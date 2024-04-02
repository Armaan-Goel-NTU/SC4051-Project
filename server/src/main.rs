use std::collections::{HashSet,HashMap};
use std::hash::Hash;
use std::net::{UdpSocket,SocketAddr};
use std::io::{Error, Read, Seek, SeekFrom, Write};
use std::process;
use std::str;
use clap::Parser;
use std::path::{Path,PathBuf};
use std::fs::File;
use dirs::{self};
use std::time::{SystemTime, UNIX_EPOCH};
use inline_colorization::*;

#[derive(Parser, Debug)]
#[command(about, long_about = None)]
struct Args {
    /// Server Port
    #[arg(short, long, default_value_t = 45600)]
    port: u16,

    /// Root File Directory
    #[arg(short, long, default_value = "")]
    dir: String,

    /// At most once semantic
    #[arg(short, long)]
    at_most_once: bool
}

#[non_exhaustive]
struct RequestOperation;
impl RequestOperation {
    pub const HANDSHAKE: u8 = 0;
    pub const DISCONNECT: u8 = 1;
    pub const READ: u8 = 2;
    pub const INSERT: u8 = 3;
    pub const UPDATE: u8 = 4;
    pub const DELETE: u8 = 5;
    pub const MONITOR: u8 = 6;
}

#[derive(Eq, Hash, PartialEq, Clone, Copy)]
struct MonitorInterval {
    addr: SocketAddr,
    end_time: u128
}

fn get_time() -> u128 {
    let start = SystemTime::now();
    let since_the_epoch = start.duration_since(UNIX_EPOCH).unwrap();
    return since_the_epoch.as_millis();
}

struct ResponseManager {
    response_map: HashMap<SocketAddr, HashMap<u32, ServerMarshal>>,
    session_map: HashMap<SocketAddr, u32>,
    at_most_once: bool
}

impl ResponseManager {
    fn flush_client(&mut self, addr: &SocketAddr) {
        if self.response_map.contains_key(addr) {
            println!("{style_bold}{color_blue}[ResponseManager]:{style_reset} Clearing response entires of this client");
            self.response_map.remove(addr);
        }

        if self.session_map.contains_key(addr) {
            println!("{style_bold}{color_blue}[ResponseManager]:{style_reset} Clearing session entry of this client");
            self.session_map.remove(addr);
        }
    }

    fn has_entry(&self, addr: &SocketAddr, req_no: u32) -> bool {
        if !self.at_most_once { return false }
        if self.response_map.contains_key(addr) {
            let req_map = self.response_map.get(addr).unwrap();
            return req_map.contains_key(&req_no);
        }
        return false
    }

    fn get_entry(&self, addr: &SocketAddr, req_no: u32) -> &ServerMarshal {
        let req_map = self.response_map.get(addr).unwrap();
        return req_map.get(&req_no).unwrap();
    }

    fn add_entry(&mut self, addr: &SocketAddr, req_no: u32, response: ServerMarshal) {
        if !self.at_most_once { return }
        let data = &response.data;
        let mut status = "GOOD";
        if response.status == StatusCode::BAD {
            status = "BAD";
        }
        println!("{style_bold}{color_blue}[ResponseManager]:{style_reset} Response has status code {status} with data '{data}'");
        println!("{style_bold}{color_blue}[ResponseManager]:{style_reset} Saving response of req no. {req_no} from this client");
        if self.response_map.contains_key(addr) {
            let req_map = self.response_map.get_mut(addr).unwrap();
            req_map.insert(req_no, response);
        } else {
            let mut req_map = HashMap::new();
            req_map.insert(req_no, response);
            self.response_map.insert(*addr, req_map);
        }
    }
}

struct MonitorManager<'a> {
    dict: HashMap<PathBuf, HashSet<MonitorInterval>>,
    socket: &'a UdpSocket
}

impl<'a> MonitorManager<'a> {
    fn add_interval(&mut self, file: PathBuf, addr: SocketAddr, interval: u32) {
        let current_time = get_time();
        let end_time = current_time + interval as u128;
        let file_str = file.to_string_lossy();
        println!("{style_bold}{color_cyan}[MonitorManager]:{style_reset} Adding monitor on {file_str} for {addr}, ending at {end_time} ({current_time} + {interval}");
        let monitor = MonitorInterval{addr,end_time};
        if self.dict.contains_key(&file) {
            self.dict.get_mut(&file).unwrap().insert(monitor);
        } else {
            let mut set: HashSet<MonitorInterval> = HashSet::new();
            set.insert(monitor);
            self.dict.insert(file, set);
        }
    }

    fn inform_monitors(&mut self, file: PathBuf, content: Vec<u8>) {
        let time : u128 = get_time();
        println!("{style_bold}{color_cyan}[MonitorManager]:{style_reset} Current time is {time}");
        let file_str = file.to_string_lossy();
        if self.dict.contains_key(&file) {
            let response = ServerMarshal{status: StatusCode::GOOD, data: str::from_utf8(&content).unwrap().to_string()};
            let set = self.dict.get_mut(&file).unwrap();
            let set_clone = set.clone();
            for element in set_clone.iter() {
                let end_time = element.end_time;
                if time > end_time {
                    let addr = element.addr;
                    println!("{style_bold}{color_cyan}[MonitorManager]:{style_reset} Removing monitor for {addr} on {file_str} set to expire at {end_time}");
                    set.remove(&element);
                } else {
                    let addr = element.addr;
                    println!("{style_bold}{color_cyan}[MonitorManager]:{style_reset} Informing {addr} of changes on {file_str} set to expire at {end_time}");
                    send(&self.socket, &response, element.addr);
                }
            }
        }
    }
}

#[non_exhaustive]
struct StatusCode;
impl StatusCode {
    pub const BAD: u8 = 0;
    pub const GOOD: u8 = 1;
}

#[derive(Clone)]
struct ServerMarshal {
    status: u8,
    data: String
}

impl ServerMarshal {
    fn into_bytes(&self) -> Vec<u8> {
        let mut buf : Vec<u8> = Vec::new();
        buf.push(self.status);
        buf.extend(self.data.as_bytes());
        return buf;
    }
}

struct ClientUnmarshal<'a> {
    buf: &'a [u8],
    i: u32
}

fn read_file(file: &mut File) -> Vec<u8> {
    let mut buf = Vec::new();
    let _ = file.seek(SeekFrom::Start(0));
    let _ = file.read_to_end(&mut buf);
    return buf;
}

impl<'a> ClientUnmarshal<'a> {
    fn read_int(&mut self) -> u32 {
        let mut c : u32 = self.buf[(self.i+3) as usize] as u32;
        c += (self.buf[(self.i+2) as usize] as u32) << 8;
        c += (self.buf[(self.i+1) as usize] as u32) << 16;
        c += (self.buf[(self.i) as usize] as u32) << 24;
        self.i += 4;
        return c;
    }

    fn read_string(&mut self) -> &str {
        let len : u32 = self.read_int();
        let mut val = "COULDN'T DECODE VALUE";
        let convert: Result<&str, str::Utf8Error> = str::from_utf8(&self.buf[self.i as usize..(self.i+len) as usize]);
        if convert.is_ok() {
            val = convert.unwrap();
        }
        self.i += len;
        return val;
    }

    fn read_byte(&mut self) -> u8 {
        let c: u8 = self.buf[self.i as usize];
        self.i += 1;
        return c;
    }

    fn parse_request(&mut self, op: u8, dir: &PathBuf, monitor_manager: &mut MonitorManager, addr: SocketAddr) -> ServerMarshal {
        let mut response =  ServerMarshal{status: StatusCode::BAD, data: "Operation Completed".to_owned()};

        match op {
            RequestOperation::READ |  RequestOperation::INSERT | RequestOperation::DELETE | RequestOperation::UPDATE | RequestOperation::MONITOR => {},
            _ => {
                println!("{style_bold}{color_magenta}[RequestHandler]:{style_reset} client requested for an invalid operation");
                response.data = "Invalid Operation".to_owned();
                return response;
            },
        }

        let file_path: &str = self.read_string();
        
        let path: PathBuf = dir.join(file_path);
        let path_str = path.to_string_lossy();
        println!("{style_bold}{color_magenta}[RequestHandler]:{style_reset} {path_str} is the file to operate on");
        if !(path.is_file() && path.exists()) {
            println!("{style_bold}{color_magenta}[RequestHandler]:{style_reset} {path_str} does not exist");
            response.data = "Invalid File Path".to_owned();
            return response;
        }

        let open: Result<File, Error> = File::options().read(true).write(true).open(&path);
        if open.is_err() {
            let err = open.err().unwrap();
            println!("{style_bold}{color_magenta}[RequestHandler]:{style_reset} {err} when opening file {path_str}");
            response.data = "Could not open file".to_owned();
            return response;
        }

        let mut file: File = open.unwrap();
        let len: u64 = file.metadata().unwrap().len();
        let mut offset: u32 = 0;

        if op != RequestOperation::MONITOR {
            offset = self.read_int();
            println!("{style_bold}{color_magenta}[RequestHandler]:{style_reset} {offset} is the offset to seek to");
            if offset as u64 >= len {
                response.data = "Offset is too large".to_owned();
                return response;
            }
        }

        let _ = file.seek(SeekFrom::Start(offset as u64));

        if op == RequestOperation::INSERT || op == RequestOperation::UPDATE {
            let data = self.read_string();
            if op == RequestOperation::UPDATE  {
                println!("{style_bold}{color_magenta}[RequestHandler]:{style_reset} client wants to overwrite '{data}' starting from the offset");
                let data_len = data.len();
                if (offset + data_len as u32) as u64 > len {
                    println!("{style_bold}{color_magenta}[RequestHandler]:{style_reset} {offset} + length of data ({data_len}) exceeds the file size ({len})");
                    response.data = "Offset+Data is too large".to_owned();
                    return response;
                }
                let _ = file.seek(SeekFrom::Start(offset as u64));
                let _ = file.write_all(data.as_bytes());
                monitor_manager.inform_monitors(path, read_file(&mut file));
            } else {
                println!("{style_bold}{color_magenta}[RequestHandler]:{style_reset} client wants to insert '{data}' at the offset");
                let mut buf = Vec::new();
                let _ = file.read_to_end(&mut buf);
                let _ = file.seek(SeekFrom::Start(offset as u64));
                let _ = file.write_all(data.as_bytes());
                let _ = file.write_all(&buf);
                monitor_manager.inform_monitors(path, read_file(&mut file));
            }
        } else if op == RequestOperation::READ || op == RequestOperation::DELETE {
            let amount: u32 = self.read_int();
            if (offset + amount) as u64 > len {
                println!("{style_bold}{color_magenta}[RequestHandler]:{style_reset} {offset} + {amount} exceeds the file size ({len})");
                response.data = "Offset+Amount is too large".to_owned();
                return response;
            } 
            if op == RequestOperation::READ {
                println!("{style_bold}{color_magenta}[RequestHandler]:{style_reset} client wants to read {amount} bytes starting from the offset");
                let mut buf = vec![0u8; amount as usize];
                let _ = file.read_exact(&mut buf);
                response.data = str::from_utf8(&buf).unwrap().to_owned();
            } else {
                println!("{style_bold}{color_magenta}[RequestHandler]:{style_reset} client wants to delete {amount} bytes starting from the offset");
                let mut buf = Vec::new();
                let _ = file.seek(SeekFrom::Start((offset + amount) as u64));
                let _ = file.read_to_end(&mut buf);
                let _ = file.seek(SeekFrom::Start(offset as u64));
                let _ = file.write_all(&buf);
                let _ = file.set_len( len - amount as u64);
                monitor_manager.inform_monitors(path, read_file(&mut file));
            }
        } else {
            let interval: u32 = self.read_int();
            println!("{style_bold}{color_magenta}[RequestHandler]:{style_reset} client has requested to monitor {path_str} for {interval}ms");
            monitor_manager.add_interval(path, addr, interval)
        }

        response.status = StatusCode::GOOD;
        return response;
    }

    fn process_request(&mut self, dir: &PathBuf, monitor_manager: &mut MonitorManager, addr: SocketAddr, response_manager: &mut ResponseManager) -> ServerMarshal {
        let req_no = self.read_int();
        let op: u8 = self.read_byte();

        if op == RequestOperation::HANDSHAKE {
            println!("{style_bold}{color_magenta}[RequestHandler]:{style_reset} client initiated handshake");
            let client_time = self.read_int();
            if response_manager.session_map.contains_key(&addr) {
                println!("{style_bold}{color_blue}[ResponseManager]:{style_reset} Found key for {client_time}");
                if response_manager.session_map.get(&addr).unwrap() != &client_time {
                    println!("{style_bold}{color_blue}[ResponseManager]:{style_reset} Clearing old responses from this client. Previous session was {client_time}");
                    response_manager.flush_client(&addr);
                    response_manager.session_map.insert(addr, client_time);
                }
            } else {
                println!("{style_bold}{color_blue}[ResponseManager]:{style_reset} Creating blank response map for this client. Session id is {client_time}");
                response_manager.session_map.insert(addr, client_time);
            }
            println!("{style_bold}{color_blue}[ResponseManager]:{style_reset} Sending handshake confirmation");
            return ServerMarshal{status: StatusCode::GOOD, data: "Handshake Completed. Welcome!".to_owned()};
        }

        if op == RequestOperation::DISCONNECT {
            println!("{style_bold}{color_magenta}[RequestHandler]:{style_reset} client wants to disconnect");
            response_manager.flush_client(&addr);
            println!("{style_bold}{color_blue}[ResponseManager]:{style_reset} Sending disconnect confirmation");
            return ServerMarshal{status: StatusCode::GOOD, data: "Bye!".to_owned()};
        }

        if response_manager.has_entry(&addr, req_no) { 
            println!("{style_bold}{color_blue}[ResponseManager]:{style_reset} Req No. {req_no} from client is a duplicate! Sending back saved response.");
            return response_manager.get_entry(&addr, req_no).clone(); 
        } else if response_manager.at_most_once {
            println!("{style_bold}{color_blue}[ResponseManager]:{style_reset} No saved response found for req no. {req_no} from client");
        }
        let response = self.parse_request(op, dir, monitor_manager, addr);
        response_manager.add_entry(&addr, req_no, response.clone());

        return response;
        
    }
}

fn send(socket: &UdpSocket, response: &ServerMarshal, addr: SocketAddr) {
    let result = socket.send_to(&response.into_bytes(), addr);
    if result.is_ok() {
        let amt = result.unwrap();
        println!("{style_bold}{color_green}[UDP]:{style_reset} Sent {amt} bytes to {addr}");
    } else {
        let err: Error = result.unwrap_err();
        println!("{style_bold}{color_green}[UDP]:{style_reset} Error Sending Data: {err}");
    }

}

const LOCALHOST: &str = "127.0.0.1";
fn main() {
    let _ = ctrlc::set_handler(move || { 
        println!("{style_bold}{color_yellow}[Server]:{style_reset} Closing"); 
        process::exit(0);
    });

    let args: Args = Args::parse();
    let dir = &args.dir;
    let mut path = Path::new(dir).to_path_buf();
    if dir.len() == 0 {
        path = dirs::home_dir().unwrap();
    }

    if !(path.is_dir() && path.exists())  {
        println!("{style_bold}{color_yellow}[Server]:{style_reset} Couldn't find server file directory!");
        process::exit(1);
    }

    let server_address:&str = &(LOCALHOST.to_owned() + ":" + &args.port.to_string());
    let socket: UdpSocket = UdpSocket::bind(server_address).expect(&format!("Couldn't bind to address {server_address}"));
    println!("{style_bold}{color_yellow}[Server]:{style_reset} Server bound to {server_address}");
    let dir_str = path.to_string_lossy();
    println!("{style_bold}{color_yellow}[Server]:{style_reset} Server file directory is {dir_str}");
    if args.at_most_once {
        println!("{style_bold}{color_yellow}[Server]:{style_reset} Using at-most-once semantics");
    } else {
        println!("{style_bold}{color_yellow}[Server]:{style_reset} Using at-least-once semantics");
    }

    let mut monitor_manager: MonitorManager<'_> = MonitorManager{dict: HashMap::new(), socket: &socket};
    let mut response_manager: ResponseManager = ResponseManager{response_map: HashMap::new(), session_map: HashMap::new(), at_most_once: args.at_most_once};

    loop {
        let mut buf: [u8; 1048576] = [0; 1024*1024];
        let result: Result<(usize, SocketAddr), Error> = socket.recv_from(&mut buf);
        if result.is_ok() {
            let (amt, src) = result.unwrap();
            println!("{style_bold}\n{color_green}[UDP]:{style_reset} Received {amt} bytes from {src}");
            let mut handler : ClientUnmarshal = ClientUnmarshal{buf: &buf, i: 0};
            let response = handler.process_request(&path, &mut monitor_manager, src, &mut response_manager);
            send(&socket, &response, src);
        } else {
            let err: Error = result.unwrap_err();
            println!("{style_bold}{color_green}[UDP]:{style_reset} Error Receiving Data: {err}");
        }
    }
}
