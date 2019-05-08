mkdir /usr/bin/supervisor-rs;
cd /usr/bin/supervisor-rs/;

if [ $(uname) = "Linux" ]
then
    rm ./supervisor-rs-client ./supervisor-rs-server && curl -L https://github.com/ccqpein/supervisor-rs/releases/latest/download/linux.zip | tar zx;
else
    rm ./supervisor-rs-client ./supervisor-rs-server && curl -L https://github.com/ccqpein/supervisor-rs/releases/latest/download/macos.zip | tar zx;
fi
