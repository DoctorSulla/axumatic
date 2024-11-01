#!/bin/bash
app_name=axumatic
pkill $app_name
cargo run &
