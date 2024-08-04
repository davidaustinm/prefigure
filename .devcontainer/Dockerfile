# Use Debian 12 as the base image
FROM debian:12

# Set environment variables
ENV DEBIAN_FRONTEND=noninteractive

# Update the package list and install necessary packages
 RUN apt-get update && \
     apt-get install -y python3-dev python3-louis python3-pip libcairo2-dev pkg-config npm nodejs && \
     apt-get clean


# Set the working directory
WORKDIR /workspaces/prefigure/js

# Install a package using npm (replace 'your-package' with the actual package name)
RUN npm install mathjax-full