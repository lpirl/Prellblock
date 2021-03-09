// use libc::c_char;
use libc;
use std::ffi::CStr;
use std::ffi::CString;
use std::ptr;
use std::mem;
use std::fmt::Write;

use lazy_static::lazy_static; // 1.4.0
use std::sync::Mutex;
use trdp_rs::*;

use crate::listener::Message;

const RESERVED_MEMORY: u32 = 2000000;
const MD_COMID1: u32 = 1001;


pub unsafe extern "C"  fn dbg_out (
    _a: *mut libc::c_void,
    category: TRDP_LOG_T,
    p_time : * const CHAR8,
    _p_file : * const CHAR8,
    _line_number : UINT16,
    p_msg_str : * const CHAR8)
{
    let cat_str = ["**Error:", "Warning:", "   Info:", "  Debug:", "   User:"];

    let c_msg: &CStr = CStr::from_ptr(p_msg_str) ;
    let msg_slice: &str = c_msg.to_str().unwrap().trim_end();
    match category {
        0 /* VOS_LOG_ERROR */ => log::error!("{}", msg_slice),
        1 /* VOS_LOG_WARNING */=> log::warn!("{}", msg_slice),
        2 /* VOS_LOG_INFO */=> log::info!("{}", msg_slice),
        3 /* VOS_LOG_DBG */ => log::trace!("{}", msg_slice),
        _ => log::error!("{} {}", cat_str[category as usize], msg_slice),
    }

}


pub fn session_id_to_string(session_id : [u8;16]) -> String {
    
    let mut s = String::new();
    for b in session_id.iter() {
        write!(&mut s, "{:X}", *b).expect("Unable to write");
    }
    
    return s;
}


pub fn message_to_string(p_data: *mut u8,data_size :u32) -> String {

    let mut s = String::new();
    for i in 0..data_size {
        unsafe {
            write!(&mut s, "{:X}", *p_data.offset(i as isize)).expect("Unable to write");
        }
    }
    
    return s;

}


pub struct Session {
    session_id : TRDP_UUID_T,
    messages : Vec<Message>,
}


impl Session {
    /// Create a new AppHandle instance.
    #[must_use]
    pub fn new(session_id : TRDP_UUID_T, src_ip : TRDP_IP_ADDR_T,dest_ip : TRDP_IP_ADDR_T) -> Self {
        Self { 
            session_id,
            messages : vec![] 
        }
    }
    
    pub fn append(&mut self, message : Message)
    {
        self.messages.push(message);
    }


    pub fn get_message(&mut self) -> Option<Message>
    {
        return self.messages.pop();
    }
}

struct AppHandle {
    handle : TRDP_APP_SESSION_T,
    sessions : Vec<Session>
}

unsafe impl Send for AppHandle {}

impl AppHandle {
     /// Create a new AppHandle instance.
     #[must_use]
     pub fn new(handle : TRDP_APP_SESSION_T) -> Self {
          
         Self {
            handle,
            sessions : vec![]
         }
     }

    pub fn last_session(&mut self) -> usize {
        return self.sessions.len() -1;
    }

    pub fn pop_session(&mut self, session_id : TRDP_UUID_T) -> Option<Session> {

        for i in 0..self.sessions.len() {
            if self.sessions[i].session_id == session_id {
                return Some(self.sessions.remove(i));
            }
        }
        return None;
    }

}

lazy_static! {
    static ref app_handles: Mutex<Vec<AppHandle>> = Mutex::new(vec![]);
}

unsafe extern "C" fn md_callback (_a: *mut libc::c_void, app_handle : TRDP_APP_SESSION_T, p_msg: *const TRDP_MD_INFO_T, p_data: *mut u8, data_size :u32)
{

    let mut app : usize = 0;
    let max : usize = app_handles.lock().unwrap().len();
    loop {
        println!("{:?} == {:?}",app_handles.lock().unwrap()[app].handle,app_handle);
        if app_handles.lock().unwrap()[app].handle == app_handle {
            break;
        }
        app = app + 1;
    }
   
    //Add temp Session
    let mut session : Session = Session::new((*p_msg).sessionId,(*p_msg).srcIpAddr,(*p_msg).destIpAddr);
    

    log::info!("Callback app-handle: {} session-id: {}", app,session_id_to_string((*p_msg).sessionId));


    /*    Check why we have been called    */
    match (*p_msg).resultCode as u32 {
        // Match a single value
        TRDP_NO_ERR => {

            //Recv message 
            let mut buf : Vec<u8> = vec![];
            for i in 0..data_size {
                buf.push(*p_data.offset(i as isize) as u8);
            }
            let message : Message = Message::new(&buf);

            //Append to Session
            session.append(message);

            match (*p_msg).msgType as u32 {
                TRDP_MSG_MR => log::info!("MR Request with reply {}",message_to_string(p_data,data_size)),
                TRDP_MSG_MP => log::info!("MD Reply without confirmation {}",message_to_string(p_data,data_size)),
                _ => panic!("message error {}",(*p_msg).msgType)
            }


        },
        _ => panic!("md_callback error {}",(*p_msg).resultCode)
    }

    app_handles.lock().unwrap()[app].sessions.push(session);

}




fn trdp_open(own_ip: TRDP_IP_ADDR_T,port : u16) -> usize {

    let mut app_handle: TRDP_APP_SESSION_T = ptr::null_mut();

    let mut handles = app_handles.lock().unwrap();


    let dynamic_config = TRDP_MEM_CONFIG_T {
        p: ptr::null_mut(),
        size: RESERVED_MEMORY,
        prealloc: [0; 15],
    };

    let host_name = [0 as i8; 17];
    let leader_name = [0 as i8; 17];

    let process_config = TRDP_PROCESS_CONFIG_T {
        hostName: host_name,
        leaderName: leader_name,
        cycleTime: 0,
        priority: 0,
        options: TRDP_OPTION_BLOCK as u8,
    };

    let md_configuration = TRDP_MD_CONFIG_T {
        pfCbFunction: Some(md_callback),     //Pointer to MD callback function -> mdCallback
        pRefCon: ptr::null_mut(), // ->  &sSessionData
        sendParam: TRDP_SEND_PARAM_T {
            qos: 0,  //Quality of service (default should be 2 for PD and 2 for MD, TSN priority >= 3)  
            ttl: 64, //Time to live (default should be 64) 
            retries: 0, //D Retries from XML file
            tsn: 0, //if TRUE, do not schedule packet but use TSN socket
            vlan: 0
        },
     
        flags: TRDP_FLAGS_CALLBACK as u8,
     
        replyTimeout: 1000000 as u32,       //Default reply timeout in us
        confirmTimeout: 1000000 as u32,     //Default confirmation timeout in us
        connectTimeout: 1000000 as u32,     //Default connection timeout in us
        sendingTimeout: 1000000 as u32,     //Default sending timeout in us

        udpPort: port,               // Port to be used for UDP MD communication (default: 17225) 
        tcpPort: port,               // Port to be used for TCP MD communication (default: 17225)

        maxNumSessions: 10 as u32,           // Maximal number of replier sessions 
    };



    let app : usize = handles.len();

    log::info!("app_handles {}",app);

    unsafe {
        if(app == 0)
        {
            let result = tlc_init(Some(dbg_out), ptr::null_mut(), &dynamic_config as *const _);
            match result {
                0 => { }
                _ => panic!("tlc_init error"),
            }
        }

        //Open Sesison
        let result = tlc_openSession(
            &mut app_handle as *mut _,
            own_ip,              /* own Ip */
            0,                  /* use default IP address    */
            ptr::null_mut(),    /* no Marshalling            */
            ptr::null_mut(),
            &md_configuration as *const _,
            &process_config as *const _,
        );

        match result {
            0 => {}
            _ => panic!("tlc_openSession error"),
        }
    }

    handles.push(AppHandle::new(app_handle));

    return app;
}

pub fn trdp_listener(own_ip: TRDP_IP_ADDR_T, port : u16) -> usize {
    

    let mut listen_tcp: TRDP_LIS_T = ptr::null_mut();
    let com_id = MD_COMID1;
    
    let app : usize = trdp_open(own_ip,port);

    log::info!("trdp_open {}",app);

    let app_handle: TRDP_APP_SESSION_T = app_handles.lock().unwrap()[app].handle;


    unsafe {    
        let result = tlm_addListener(
            app_handle, 
            &mut listen_tcp as *mut _,
            ptr::null_mut(), 
            Some(md_callback),
            1, //true
            com_id,
            0, 0, 
            libc::INADDR_ANY,
            libc::INADDR_ANY,         /*    Source IP filter              */
            own_ip,
            (/* TRDP_FLAGS_TCP | */ TRDP_FLAGS_CALLBACK) as u8, ptr::null_mut(), ptr::null_mut());
        match result {
            0 => {}
            _ => panic!("tlm_reply error"),
        }
    }
    return app;
}

fn trdp_handle(app_handle: TRDP_APP_SESSION_T)
{
    log::info!("trdp_handle app: {:?}",app_handle);

    unsafe {

        let mut rfds = mem::MaybeUninit::<libc::fd_set>::uninit();

        let mut no_desc: i32 = 0;
        let mut tv = TRDP_TIME_T {
            tv_sec: 0,
            tv_usec: 0,
        };
        let max_tv = TRDP_TIME_T {
            tv_sec: 1,
            tv_usec: 0,
        };
        let min_tv = TRDP_TIME_T {
            tv_sec: 0,
            tv_usec: TRDP_PROCESS_DEFAULT_CYCLE_TIME as i64,
        };

        libc::FD_ZERO(rfds.as_mut_ptr());
        let mut rfds = rfds.assume_init();

        tlc_getInterval(app_handle, &mut tv, &mut rfds, &mut no_desc);

        if vos_cmpTime(&tv, &max_tv) > 0 {
            tv = max_tv;
        }

        if vos_cmpTime(&tv, &min_tv) < 0 {
            tv = min_tv;
        }

        let mut rv = vos_select(
            no_desc + 1,
            &mut rfds,
            ptr::null_mut(),
            ptr::null_mut(),
            &mut tv,
        );

        tlm_process(app_handle, &mut rfds, &mut rv);
    }
}

pub fn trdp_accept(app : usize) -> tokio::task::JoinHandle<Session> {

    return tokio::task::spawn_blocking(move || {
        loop {


            let app_handle: TRDP_APP_SESSION_T = app_handles.lock().unwrap()[app].handle;
            trdp_handle(app_handle);
    
            let result = app_handles.lock().unwrap()[app].sessions.pop();
    
    
            match result {
                None => {},
                Some(session) => {
                    log::info!("Accept new Session {} {:?}",app,session.session_id);
                    return session;
                }
            } 
        }
    });
    
    
}

pub fn trdp_wait_response(app : usize,session_id : TRDP_UUID_T) -> Session {

    let app_handle: TRDP_APP_SESSION_T = app_handles.lock().unwrap()[app].handle;

    loop {
        trdp_handle(app_handle);
       
        match app_handles.lock().unwrap()[app].pop_session(session_id) {
            None => {},
            Some(session) => { return session },
        }

    }
    
}

pub fn trdp_connect(own_ip: TRDP_IP_ADDR_T,port : u16) -> usize {

    let com_id = MD_COMID1;
    
    let app : usize = trdp_open(own_ip,port);

    log::info!("Connect {}",app);

    return app;
}


pub fn trdp_disconnect(app: usize) {

    log::info!("Disconnect {}",app);

    /*
    let app_handle: TRDP_APP_SESSION_T = app_handles.lock().unwrap()[app].handle;
    unsafe {
        let result = tlc_closeSession(app_handle);
        match result {
            0 => {}
            _ => panic!("tlc_closeSession error {}",result),
        }
    }
    */
}


pub fn trdp_send_reply(app: usize,session : &Session,message : &Message) 
{
    let app_handle: TRDP_APP_SESSION_T = app_handles.lock().unwrap()[app].handle;

    let buf = message.to_buffer();
    let com_id = MD_COMID1;


    let mut src_uri = CString::new("md_listener").unwrap().into_bytes_with_nul();
    unsafe {
        let result = tlm_reply(app_handle,
            &session.session_id,
            com_id,
            0,
            ptr::null_mut(),
            buf.as_ptr() as *const u8,
            buf.len() as u32,
            src_uri.as_mut_ptr() as *mut i8);
        match result {
            0 => {}
            _ => panic!("tlm_reply error {}",result),
        }
    }



}

pub fn trdp_send_request(app : usize,dest_ip: TRDP_IP_ADDR_T,message : &Message) -> TRDP_UUID_T
{
    log::info!("Send request {}",app);

    let app_handle: TRDP_APP_SESSION_T = app_handles.lock().unwrap()[app].handle;

    let mut session_id : TRDP_UUID_T =  [0; 16];
    let delay : u32 = 20000000;  //2000000  TODO why 2000000 is not sufficient

    let buf = message.to_buffer();
    let com_id = MD_COMID1;

    unsafe {
        let result = tlm_request(app_handle,
            ptr::null_mut(),
            Some(md_callback),
            &mut session_id as *mut _,
            com_id,
            0,
            0,
            0,
            dest_ip,
            (/* TRDP_FLAGS_TCP |*/ TRDP_FLAGS_CALLBACK) as u8,
            1, //expReplies
            delay, //delay
            ptr::null_mut(),
            buf.as_ptr() as *const u8,
            buf.len() as u32,
            ptr::null_mut(), 
            ptr::null_mut());

        match result {
            0 => {}
            _ => panic!("tlm_request error"),
        }
    }
    
    return session_id;
}


