# Metaquity Network
​
The World's First Permissioned Blockchain Network and DeFi Protocol for Real World Assets (RWA) focusing on solar assets.
​
## Building
​
1. Clone the repository
​
    ```console
    git clone https://github.com/Metaquity-Network/metaquity-network-node && cd metaquity-network-node
    ```
​
2. Setup Rust if you don't have it yet
​
    ```console
    sh scripts/init.sh
    ```
​
3. Build the node binary
​
    ```console
    cargo build --release
    ```
​
4. Build a Docker image
​
    ```console
    cp target/release/
    docker build -t metaquity-network .
    ```
​
## Launch a collator node
​
1. Export the Docker image from the build environment
​
    ```console
    docker image save metaquity-network:latest -o metaquity-network.img
    ```
​
2. Copy the image to the server
​
    ```console
    scp metaquity-network.img $REMOTE_USER@$REMOTE_SERVER:~
    ```
​
3. Login to the server and load the image from the build environment
​
    ```console
    ssh $REMOTE_USER@$REMOTE_SERVER
    docker image load -i metaquity-network.img
    ```
​
4. Launch the collator node in container
​
    ```console
    docker run -d \
        --name metaquity-network-collator \
        --restart unless-stopped \
        --network host \
        -v metaquity-rococo-testnet-collator-data:/data \
        metaquity-network:latest \
        metaquity-network \
        --collator \
        --base-path /data \
        --port 50333 \
        --rpc-port 8855 \
        --bootnodes /dns/3.aws.metaquity.xyz/tcp/50333/p2p/12D3KooWAXpHoi3P7B1aEmdoVYMunctQUtWJwfpbNRbpLwREQQM2 \
        -- \
        --chain rococo \
        --port 50343 \
        --rpc-port 9988
    ```
