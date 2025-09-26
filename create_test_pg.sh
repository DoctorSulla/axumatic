#!/bin/bash
sudo docker run --name axumatic-test-postgres -e POSTGRES_PASSWORD=mysecretpassword -d -p 5432:5432 postgres
