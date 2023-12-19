# Create a Ubuntu Trusty (14.04) Docker Image

This Docker image should have all of the dependencies needed to build the code in this repository for Ubuntu Trusty (14.04) and above.

## Prerequisites
You will need to have Docker installed on your machine. If you do not have Docker installed, you can find instructions on how to install it [here](https://docs.docker.com/engine/installation/).

## Building the Image
Simply run the following command to create the image. If you are also planning to use the image to build the code in this repository, you should also tag the image with the name `trusty-ubuntu-audio-image` (as shown below) so that the build scripts (later on) can find it.

```bash
docker build -t trusty-ubuntu-audio-image -f Dockerfile .
```