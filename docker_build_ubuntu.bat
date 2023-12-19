:: PLEASE RUN THE .\create_base_ubuntu_docker_image\ script first to create the base image
:: Check out the README.md for more information before proceeding

docker run --name audio-server-build-temp -it -v "%cd%:/app" trusty-ubuntu-audio-image sh -c "cd /app && cargo build --release --target=x86_64-unknown-linux-gnu" && docker rm audio-server-build-temp