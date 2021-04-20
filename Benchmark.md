
## Path 
set absolute path if you are testing with devcontaineron Windows OS

export SRCPATH=/c/git/PrellblockBenchmarking

## Run blockchain, prometheus and grafana
docker-compose up --build

## send messages

docker exec -it prellblockbench_emily_1 sh
/prellblock # export RUST_LOG=info

### send single value
/prellblock/target/release/prellblock-client config/temperature-1/temperature-1.key emily:3130 set hello world

### send muchos values
/prellblock/target/release/prellblock-client config/temperature-1/temperature-1.key emily:3130 bench emily 20000 --size 128


### cleanup

rm -rf data
rm -rf blocks