export PORT=${1:-9000}

systemfd --no-pid -s http::$PORT -- cargo watch -x run
