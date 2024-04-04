# SC4051-Project
Remote File Access System for SC4051 Distributed Systems

# Installation
The client is written in Go, which can be installed from https://go.dev/doc/install \
The server is written in Rust, which can be installed from https://www.rust-lang.org/tools/install

# Usage
## Client
```
cd client
go run client
```
Passing arguments:
```
go run client -help
go run client -host=192.168.0.1 -t=60000
```

Alternatively, you can build the program
```
cd client
go build client
./client -help
./client -host=192.168.0.1 -t=60000
```

## Server
```
cd server
cargo run
```

Passing arguments:
```
cargo run server -- --help
cargo run server -p 44444 --at-most-once
```

Alternatively, you can build the program
```
cd server
cargo build --release
cd target/release
./server --help
./server -p 44444 --at-most-once
```
