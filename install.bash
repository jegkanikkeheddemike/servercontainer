cargo build --release
mkdir -p /home/$USER/.servercontainer/bin
cp ./target/release/servercontainer /home/$USER/.servercontainer/bin/servercontainer
cat ./path >> /home/$USER/.bashrc
