# Use an official Python runtime as a parent image
FROM python:3.11

# Set the working directory in the container
WORKDIR /usr/src/app

# Install any needed packages specified in requirements.txt
COPY requirements.txt ./
RUN pip install --no-cache-dir -r requirements.txt

# Copy the rest of your application's code
COPY . .

# Define environment variable
ENV NAME World

# Run the application
CMD ["python", "./your-script.py"]
