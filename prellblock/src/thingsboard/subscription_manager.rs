use super::subscriptions::SubscriptionConfig;
use crate::{block_storage, block_storage::BlockStorage, consensus::Error};
use http::StatusCode;
use pinxit::PeerId;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, env, time::Duration};
use tokio::time::delay_until;

const ENV_THINGSBOARD_USERNAME: &str = "THINGSBOARD_USER_NAME";
const ENV_THINGSBOARD_PASSWORD: &str = "THINGSBOARD_PASSWORD";
const ENV_THINGSBOARD_TENANT_ID: &str = "THINGSBOARD_TENANT_ID";

/// Manages subscriptions of timeseries.
#[derive(Debug)]
pub struct SubscriptionManager {
    block_storage: BlockStorage,
    subscriptions: HashMap<PeerId, (HashMap<String, ()>, String)>,
    http_client: reqwest::Client,
    user_token: String,
}

impl SubscriptionManager {
    /// This creates a new `SubscriptionManager`.
    pub async fn new(block_storage: BlockStorage) -> Self {
        let mut subscriptions: HashMap<PeerId, (HashMap<String, ()>, _)> = HashMap::new();
        let config = SubscriptionConfig::load();
        for subscription in config.subscription {
            if let Some((peer_map, _)) = subscriptions.get_mut(&subscription.peer_id) {
                peer_map.insert(subscription.namespace, ());
            } else {
                let mut peer_map = HashMap::new();
                peer_map.insert(subscription.namespace, ());
                subscriptions.insert(subscription.peer_id, (peer_map, subscription.device_type));
            }
        }

        let user_token = loop {
            match Self::user_token().await {
                Ok(user_token) => {
                    break user_token;
                }
                Err(err) => {
                    log::warn!("Error while retrieving the thingsboard user token: {}", err);
                    delay_until(tokio::time::Instant::now() + Duration::from_secs(1)).await;
                }
            }
        };

        let manager = Self {
            block_storage,
            subscriptions,
            http_client: reqwest::Client::new(),
            user_token,
        };

        loop {
            match manager.setup_devices().await {
                Ok(()) => {
                    break;
                }
                Err(err) => {
                    log::warn!("Error while setting up thingsboard devices: {}", err);
                }
            }
            delay_until(tokio::time::Instant::now() + Duration::from_secs(1)).await;
        }

        log::info!("SubscriptionManager setup successfully.");
        manager
    }

    /// This creates devices in thingsboard for each accessToken in the subscriptions.
    pub async fn setup_devices(&self) -> Result<(), Error> {
        let client = reqwest::Client::new();

        #[allow(clippy::single_match_else)]
        let tenant_id = match env::var(ENV_THINGSBOARD_TENANT_ID) {
            Ok(tenant_id) => tenant_id,
            Err(_) => {
                return Err(Error::ThingsboardTenantIdNotSet);
            }
        };

        let mut counter = 0;
        for (peer_id, (_, device_type)) in &self.subscriptions {
            // create a new device via POST
            let url = format!("http://localhost:8080/api/device?accessToken={}", peer_id);

            let body = build_thingsboard_device(
                format!("Sensor{}", counter).to_string(),
                device_type.to_string(),
                tenant_id.clone(),
            );

            let request = client
                .post(&url)
                .header("Content-Type", "application/json")
                .header(
                    "X-Authorization",
                    format!("Bearer:{}", self.user_token).to_string(),
                )
                .body(body.to_string());

            let response = request.send().await?;

            println!("device: {}", response.text().await?);

            counter += 1;
        }
        Ok(())
    }

    async fn user_token() -> Result<String, Error> {
        #[derive(Deserialize, Debug)]
        struct Tokens {
            token: String,
            refreshToken: String,
        }

        // Get the environment variables for the thingboard account and password
        #[allow(clippy::single_match_else)]
        let thingsboard_username = match env::var(ENV_THINGSBOARD_USERNAME) {
            Ok(username) => username,
            Err(_) => {
                return Err(Error::ThingsboardUserNameNotSet);
            }
        };

        #[allow(clippy::single_match_else)]
        let thingsboard_password = match env::var(ENV_THINGSBOARD_PASSWORD) {
            Ok(password) => password,
            Err(_) => {
                return Err(Error::ThingsboardPasswordNotSet);
            }
        };

        let body = serde_json::json!({ "username": thingsboard_username, "password":thingsboard_password });

        let url = "http://localhost:8080/api/auth/login";

        let request = reqwest::Client::new()
            .post(url)
            .header("Content-Type", "application/json")
            .body(body.to_string());

        let response = request.send().await?.text().await?;
        let response: Tokens = serde_json::from_str(&response).unwrap();
        Ok(response.token)
    }

    /// This will be called on an Applay-`Block` event.
    pub async fn notify_block_update(
        &self,
        data: Vec<(PeerId, String)>,
    ) -> Result<(), block_storage::Error> {
        for (peer_id, namespace) in data {
            if let Some((peer_map, _)) = self.subscriptions.get(&peer_id) {
                if peer_map.contains_key(&namespace) {
                    // get transaction from block_storage
                    let transaction = self.block_storage.read_transaction(&peer_id, &namespace)?;
                    // post transaction to thingsboard
                    if let Some((_, value)) = transaction.iter().next() {
                        self.post_value(&value.0, &namespace, &peer_id.to_string())
                            .await;
                    }
                }
            }
        }
        Ok(())
    }
    async fn post_value(&self, value: &[u8], namespace: &str, access_token: &str) {
        let url = thingsboard_url(access_token);
        let value: f64 = postcard::from_bytes(value).unwrap();
        let key_value_json = format!("{{{}:{}}}", namespace, value);
        log::trace!("Sending POST w/ json body: {}", key_value_json);
        let body = self
            .http_client
            .post(&url)
            .header("Content-Type", "application/json")
            .body(key_value_json);
        //send request
        let res = body.send().await;
        match res {
            Ok(res) => match res.status() {
                StatusCode::OK => log::trace!("Send POST successfully."),
                StatusCode::BAD_REQUEST => log::warn!("BAD_REQUEST response from {}.", url),
                _ => {
                    log::trace!("Statuscode: {:?}", res.status());
                }
            },
            Err(err) => {
                log::error!("{}", err);
            }
        }
    }
}

fn thingsboard_url(access_token: &str) -> String {
    let host = "localhost";
    let port = "8080";
    format!("http://{}:{}/api/v1/{}/telemetry", host, port, access_token)
}

/// This creates a thingsboard device json configuration.
fn build_thingsboard_device(device_name: String, device_type: String, tenant_id: String) -> String {
    // {
    //     "name":"TestSensorA",
    //     "tenantId": {
    //         "entityType": "TENANT",
    //         "id": "bacon"
    //     },
    //     "entityType": "DEVICE",
    //     "type": "prellblock-sensor"
    // }
    #[derive(Serialize, Deserialize)]
    struct Tenant {
        #[allow(non_snake_case)]
        entityType: String,

        id: String,
    }
    #[derive(Serialize, Deserialize)]
    struct Device {
        #[allow(non_snake_case)]
        name: String,

        #[allow(non_snake_case)]
        tenantId: Tenant,

        #[allow(non_snake_case)]
        entityType: String,

        r#type: String,
    }

    let device = Device {
        name: device_name,
        tenantId: Tenant {
            entityType: "TENANT".to_string(),
            id: tenant_id,
        },
        entityType: "DEVICE".to_string(),
        r#type: device_type,
    };

    serde_json::json!(device).to_string()
}
