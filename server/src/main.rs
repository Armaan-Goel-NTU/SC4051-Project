use std::collections::{HashSet,HashMap};
use std::hash::Hash;
use std::net::{UdpSocket,SocketAddr};
use std::io::{Error, Read, Seek, SeekFrom, Write};
use std::process;
use std::str;
use clap::Parser;
use std::path::{Path,PathBuf};
use std::fs::File;
use dirs;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Parser, Debug)]
#[command(about, long_about = None)]
struct Args {
    /// Server Port
    #[arg(short, long, default_value_t = 45600)]
    port: u16,

    /// Root File Directory
    #[arg(short, long, default_value = "")]
    dir: String
}

#[non_exhaustive]
struct RequestOperation;
impl RequestOperation {
    pub const READ: u8 = 0;
    pub const INSERT: u8 = 1;
    pub const UPDATE: u8 = 2;
    pub const DELETE: u8 = 3;
    pub const MONITOR: u8 = 4;
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

struct MonitorManager<'a> {
    dict: HashMap<PathBuf, HashSet<MonitorInterval>>,
    socket: &'a UdpSocket
}

impl<'a> MonitorManager<'a> {
    fn add_interval(&mut self, file: PathBuf, addr: SocketAddr, interval: u32) {
        let monitor = MonitorInterval{addr, end_time: get_time() + interval as u128};
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
        if self.dict.contains_key(&file) {
            let response = ServerMarshal{status: StatusCode::GOOD, data: str::from_utf8(&content).unwrap().to_string()};
            let set = self.dict.get_mut(&file).unwrap();
            let set_clone = set.clone();
            for element in set_clone.iter() {
                if time > element.end_time {
                    set.remove(&element);
                } else {
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

    fn process_request(&mut self, dir: &PathBuf, monitor_manager: &mut MonitorManager, addr: SocketAddr) -> ServerMarshal {
        let mut response =  ServerMarshal{status: StatusCode::BAD, data: "Operation Completed".to_owned()};

        match self.buf[0] {
            RequestOperation::READ |  RequestOperation::INSERT | RequestOperation::DELETE | RequestOperation::UPDATE | RequestOperation::MONITOR => {},
            _ => {
                response.data = "Invalid Operation".to_owned();
                return response;
            },
        }

        self.i += 1;
        let op: u8 = self.buf[0];
        let file_path: &str = self.read_string();
        
        let path: PathBuf = dir.join(file_path);
        if !(path.is_file() && path.exists()) {
            response.data = "Invalid File Path".to_owned();
            return response;
        }

        let open: Result<File, Error> = File::options().read(true).write(true).open(&path);
        if open.is_err() {
            response.data = "Could not open file".to_owned();
            return response;
        }

        let mut file: File = open.unwrap();
        let len: u64 = file.metadata().unwrap().len();
        let mut offset: u32 = 0;

        if op != RequestOperation::MONITOR {
            offset = self.read_int();
            if offset as u64 >= len {
                response.data = "Offset is too large".to_owned();
                return response;
            }
        }

        let _ = file.seek(SeekFrom::Start(offset as u64));

        if op == RequestOperation::INSERT || op == RequestOperation::UPDATE {
            let data = self.read_string();
            if op == RequestOperation::UPDATE  {
                if (offset + data.len() as u32) as u64 > len {
                    response.data = "Offset+Data is too large".to_owned();
                    return response;
                }
                let _ = file.seek(SeekFrom::Start(offset as u64));
                let _ = file.write_all(data.as_bytes());
                monitor_manager.inform_monitors(path, read_file(&mut file));
            } else {
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
                response.data = "Offset+Amount is too large".to_owned();
                return response;
            } 
            if op == RequestOperation::READ {
                let mut buf = vec![0u8; amount as usize];
                let _ = file.read_exact(&mut buf);
                response.data = str::from_utf8(&buf).unwrap().to_owned();
            } else {
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
            monitor_manager.add_interval(path, addr, interval)
        }

        response.status = StatusCode::GOOD;
        return response;
        
    }
}

fn send(socket: &UdpSocket, response: &ServerMarshal, addr: SocketAddr) {
    let result = socket.send_to(&response.into_bytes(), addr);
    if result.is_ok() {
        let amt = result.unwrap();
        println!("Sent {amt} bytes");
    } else {
        let err: Error = result.unwrap_err();
        println!("Error Sending Data: {err}");
    }

}

const LOCALHOST: &str = "127.0.0.1";
fn main() {
    let args: Args = Args::parse();
    let dir = &args.dir;
    let mut path = Path::new(dir).to_path_buf();
    if dir.len() == 0 {
        path = dirs::home_dir().unwrap();
    }

    if !(path.is_dir() && path.exists())  {
        println!("Couldn't find server file directory!");
        process::exit(1);
    }

    let server_address:&str = &(LOCALHOST.to_owned() + ":" + &args.port.to_string());
    let socket: UdpSocket = UdpSocket::bind(server_address).expect(&format!("Couldn't bind to address {server_address}"));
    let mut monitor_manager: MonitorManager<'_> = MonitorManager{dict: HashMap::new(), socket: &socket};

    loop {
        let mut buf: [u8; 1048576] = [0; 1024*1024];
        let result: Result<(usize, SocketAddr), Error> = socket.recv_from(&mut buf);
        if result.is_ok() {
            let (amt, src) = result.unwrap();
            println!("Amount: {amt}. Source: {src}");
            let mut handler : ClientUnmarshal = ClientUnmarshal{buf: &buf, i: 0};
            let response = handler.process_request(&path, &mut monitor_manager, src);
            send(&socket, &response, src);
        } else {
            let err: Error = result.unwrap_err();
            println!("Error Receiving Data: {err}");
        }
    }
}
