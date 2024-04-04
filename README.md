# SC4051-Project
Remote File Access System for SC4051 Distributed Systems

# Installation
The client is written in Go, which can be installed from https://go.dev/doc/install \
The server is written in Rust, which can be installed from https://www.rust-lang.org/tools/install

# Usage
## Client
```
  -c_port int
    	client port (default 45601)
  -host string
    	server host (default "127.0.0.1")
  -retries int
    	number of request retries (default 3)
  -s_port int
    	server port (default 45600)
  -t int
    	freshness interval (default 10000)
  -timeout int
    	response timeout (default 3000)
```

To build & run the program
```
cd client
go build client
./client -help
./client -host=192.168.0.1 -t=60000
```

## Server
```
Usage: server [OPTIONS]

Options:
  -p, --port <PORT>   Server Port [default: 45600]
  -d, --dir <DIR>     Root File Directory [default: ]
  -a, --at-most-once  At most once semantic
  -h, --help          Print help
```

To build & run the program
```
cd server
cargo build --release
cd target/release
./server --help
./server -p 44444 --at-most-once
```
