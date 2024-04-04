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

/* this defines the arguments that the server accepts
   Parser is derived from clap */
#[derive(Parser, Debug)]
#[command(about, long_about = None)]
struct Args {
    /// Server Host
    #[arg(short, long, default_value = "localhost")]
    server_host: String,

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

// Operation constants for easy access and modification if need be
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

// A struct to represent the response that is sent to clients
#[derive(Clone)]
struct ResponseMarshal {
    status: u8,
    data: String
}

// Converts the struct into a byte buffer for sending via UDP
impl ResponseMarshal {
    fn into_bytes(&self) -> Vec<u8> {
        let mut buf : Vec<u8> = Vec::new();
        buf.push(self.status);
        buf.extend(self.data.as_bytes());
        return buf;
    }
}

/* ResponseManager keeps a track of the responses generated for each client
   as well as the session id associated with them for at-most-once semantics */
struct ResponseManager {
    response_map: HashMap<SocketAddr, HashMap<u32, ResponseMarshal>>, // map client to another map that maps each request number with the response generated for it
    session_map: HashMap<SocketAddr, u32>, // map client to session id
}

impl ResponseManager {
    // clear out any entries belonging to a certain client
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

    // check if a response entry for a request number exists for a specific client
    fn has_entry(&self, addr: &SocketAddr, req_no: u32) -> bool {
        if self.response_map.contains_key(addr) {
            let req_map = self.response_map.get(addr).unwrap();
            return req_map.contains_key(&req_no);
        }
        return false
    }

    // return the entry (it is assumed that has_entry is checked before calling this)
    fn get_entry(&self, addr: &SocketAddr, req_no: u32) -> &ResponseMarshal {
        let req_map = self.response_map.get(addr).unwrap();
        return req_map.get(&req_no).unwrap();
    }

    // Add a response to the map
    fn add_entry(&mut self, addr: &SocketAddr, req_no: u32, response: ResponseMarshal) {
        println!("{style_bold}{color_blue}[ResponseManager]:{style_reset} Saving response of req no. {req_no} from this client");
        if self.response_map.contains_key(addr) {
            // add to existing map
            let req_map = self.response_map.get_mut(addr).unwrap();
            req_map.insert(req_no, response);
        } else {
            // create an entry for the client and then add the response to it
            let mut req_map = HashMap::new();
            req_map.insert(req_no, response);
            self.response_map.insert(*addr, req_map);
        }
    }
}

// A struct to represent a monitor interval
#[derive(Eq, Hash, PartialEq, Clone, Copy)]
struct MonitorInterval {
    addr: SocketAddr, // this is the destination client socket which will receive data
    end_time: u128
}

// The monitor manager maintains a set of intervals for each file
struct MonitorManager<'a> {
    dict: HashMap<PathBuf, HashSet<MonitorInterval>>,
    socket: &'a UdpSocket // this is the locally bound source socket from which we will send data
}

impl<'a> MonitorManager<'a> {
    // used to get the current time as epoch milliseconds
    fn get_time(&self) -> u128 {
        let start = SystemTime::now();
        let since_the_epoch = start.duration_since(UNIX_EPOCH).unwrap();
        return since_the_epoch.as_millis();
    }
    
    // adds a monitor interval to the map
    fn add_interval(&mut self, file: PathBuf, addr: SocketAddr, interval: u32) {
        let current_time = self.get_time();
        // the monitor time starts now will expire when we pass the interval
        let end_time = current_time + interval as u128;
        let file_str = file.to_string_lossy();
        println!("{style_bold}{color_cyan}[MonitorManager]:{style_reset} Adding monitor on {file_str} for {addr}, ending at {end_time} ({current_time} + {interval}");
        let monitor = MonitorInterval{addr,end_time};
        if self.dict.contains_key(&file) {
            // add to existing map
            self.dict.get_mut(&file).unwrap().insert(monitor);
        } else {
            // create an entry for the client and then add the response to it
            let mut set: HashSet<MonitorInterval> = HashSet::new();
            set.insert(monitor);
            self.dict.insert(file, set);
        }
    }

    // function responsible for checking if clients must be informed of changes as well as clearing expired monitor entries
    fn inform_monitors(&mut self, file: PathBuf, content: Vec<u8>) {
        let time : u128 = self.get_time();
        let file_str = file.to_string_lossy();
        println!("{style_bold}{color_cyan}[MonitorManager]:{style_reset} Checking if any clients must be informed about changes on {file_str}. Current time is {time}");
        if self.dict.contains_key(&file) {
            let response = ResponseMarshal{status: StatusCode::GOOD, data: str::from_utf8(&content).unwrap().to_string()};
            let set = self.dict.get_mut(&file).unwrap();
            // use a clone of the set for iteration as we will be removing some elements
            let set_clone = set.clone(); 
            for element in set_clone.iter() {
                let end_time = element.end_time;
                if time > end_time {
                    // monitor has expired and will be removed
                    let addr = element.addr;
                    println!("{style_bold}{color_cyan}[MonitorManager]:{style_reset} Removing monitor for {addr} on {file_str} set to expire at {end_time}");
                    set.remove(&element);
                } else {
                    // monitor is valid and thus the client who this monitor belongs to is informed
                    let addr = element.addr;
                    println!("{style_bold}{color_cyan}[MonitorManager]:{style_reset} Informing {addr} of changes on {file_str} set to expire at {end_time}");
                    send(&self.socket, &response, element.addr);
                }
            }
        }
    }
}

// Status type defined here and corresponds to what the client has
#[non_exhaustive]
struct StatusCode;
impl StatusCode {
    pub const BAD: u8 = 0;
    pub const GOOD: u8 = 1;
}

struct RequestHandler<'a> {
    buf: &'a [u8], // this is the data received from a client
    i: u32 // this is the current index in the buffer when unmarshalling
}

impl<'a> RequestHandler<'a> {
    // reads a file in its entirety for monitoring
    fn read_file(&self, file: &mut File) -> Vec<u8> {
        let mut buf = Vec::new();
        let _ = file.seek(SeekFrom::Start(0));
        let _ = file.read_to_end(&mut buf);
        return buf;
    }
    
    // unmarshalls an integer from the buffer
    fn read_int(&mut self) -> u32 {
        // these operations are a reverse of what the client does
        let mut c : u32 = self.buf[(self.i+3) as usize] as u32;
        c += (self.buf[(self.i+2) as usize] as u32) << 8;
        c += (self.buf[(self.i+1) as usize] as u32) << 16;
        c += (self.buf[(self.i) as usize] as u32) << 24;
        self.i += 4;
        return c;
    }

    // unmarshalls a string from the buffer
    fn read_string(&mut self) -> &str {
        // read the length first
        let len : u32 = self.read_int();
        
        // a decode error really shouldn't happen here. but just in case
        let mut val = "COULDN'T DECODE VALUE";
        let convert: Result<&str, str::Utf8Error> = str::from_utf8(&self.buf[self.i as usize..(self.i+len) as usize]);
        if convert.is_ok() {
            val = convert.unwrap();
        }

        // increment by the number of bytes read
        self.i += len;
        return val;
    }

    // used to read 1 byte from the buffer. used for reading the operation type
    fn read_byte(&mut self) -> u8 {
        let c: u8 = self.buf[self.i as usize];
        self.i += 1;
        return c;
    }

    // this function handles user services (read, insert, delete, update, monitor) if there is no saved response
    fn parse_request(&mut self, op: u8, dir: &PathBuf, monitor_manager: &mut MonitorManager, addr: SocketAddr) -> ResponseMarshal {
        let mut response =  ResponseMarshal{status: StatusCode::BAD, data: "Operation Completed".to_owned()};

        // just in case an incorrect service request is received
        match op {
            RequestOperation::READ |  RequestOperation::INSERT | RequestOperation::DELETE | RequestOperation::UPDATE | RequestOperation::MONITOR => {},
            _ => {
                println!("{style_bold}{color_magenta}[RequestHandler]:{style_reset} client requested for an invalid operation");
                response.data = "Invalid Operation".to_owned();
                return response;
            },
        }

        // all services have a file path at the start
        let file_path: &str = self.read_string();
        
        let path: PathBuf = dir.join(file_path);
        let path_str = path.to_string_lossy();
        println!("{style_bold}{color_magenta}[RequestHandler]:{style_reset} {path_str} is the file to operate on");

        // check if file exits
        if !(path.is_file() && path.exists()) {
            println!("{style_bold}{color_magenta}[RequestHandler]:{style_reset} {path_str} does not exist");
            response.data = "Invalid File Path".to_owned();
            return response;
        }

        // an error opening a file shouldn't happen, but just in case
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

        // all requests except monitor have an offset included next
        if op != RequestOperation::MONITOR {
            offset = self.read_int();
            println!("{style_bold}{color_magenta}[RequestHandler]:{style_reset} {offset} is the offset to seek to");
            // offset should not go beyond the length of the feel
            if offset as u64 >= len {
                response.data = "Offset is too large".to_owned();
                return response;
            }
        }

        // seek to the specified offset
        let _ = file.seek(SeekFrom::Start(offset as u64));

        if op == RequestOperation::INSERT || op == RequestOperation::UPDATE {
            // insert and update both contain string data
            let data = self.read_string();
            if op == RequestOperation::UPDATE  {
                println!("{style_bold}{color_magenta}[RequestHandler]:{style_reset} client wants to overwrite '{data}' starting from the offset");
                let data_len = data.len();
                // if the given string extends beyond the length of the file, it is treated as an error. no insertion will happen
                if (offset + data_len as u32) as u64 > len {
                    println!("{style_bold}{color_magenta}[RequestHandler]:{style_reset} {offset} + length of data ({data_len}) exceeds the file size ({len})");
                    response.data = "Offset+Data is too large".to_owned();
                    return response;
                }
                // seek and overwrite data
                let _ = file.seek(SeekFrom::Start(offset as u64));
                let _ = file.write_all(data.as_bytes());

                // file has been changed, check if other clients need to be informed
                monitor_manager.inform_monitors(path, self.read_file(&mut file));
            } else {
                println!("{style_bold}{color_magenta}[RequestHandler]:{style_reset} client wants to insert '{data}' at the offset");
                let mut buf = Vec::new();
                // since we have seeked to the offset, we'll read the file to end from there and save into a buffer
                let _ = file.read_to_end(&mut buf);
                // we then seek back to the offset
                let _ = file.seek(SeekFrom::Start(offset as u64));
                // we write the data to insert
                let _ = file.write_all(data.as_bytes());
                // followed by the previous content that is saved, essentially shifting it to the right
                let _ = file.write_all(&buf);
                // file has been changed, check if other clients need to be informed
                monitor_manager.inform_monitors(path, self.read_file(&mut file));
            }
        } else if op == RequestOperation::READ || op == RequestOperation::DELETE {
            // read and delete both contain an integer amount
            let amount: u32 = self.read_int();
            // if the given offset + amount extends beyond the length of the file, it is treated as an error. 
            if (offset + amount) as u64 > len {
                println!("{style_bold}{color_magenta}[RequestHandler]:{style_reset} {offset} + {amount} exceeds the file size ({len})");
                response.data = "Offset+Amount is too large".to_owned();
                return response;
            } 
            if op == RequestOperation::READ {
                println!("{style_bold}{color_magenta}[RequestHandler]:{style_reset} client wants to read {amount} bytes starting from the offset");
                let mut buf = vec![0u8; amount as usize];
                // reads the exact size of the buffer which is set to the given amount
                let _ = file.read_exact(&mut buf);
                // convert into a string serving as the response to the client
                response.data = str::from_utf8(&buf).unwrap().to_owned();
            } else {
                println!("{style_bold}{color_magenta}[RequestHandler]:{style_reset} client wants to delete {amount} bytes starting from the offset");
                let mut buf = Vec::new();
                // we additionally seek by the amount from the current position which is the offset
                let _ = file.seek(SeekFrom::Current(amount as i64));
                // we'll read the file to end from here and save into a buffer
                let _ = file.read_to_end(&mut buf);
                // we'll seek back to the offset from the start
                let _ = file.seek(SeekFrom::Start(offset as u64));
                // and overwrite it with out previously saved buffer, essentially shifting it to the left
                let _ = file.write_all(&buf);
                // and set a new length for the file
                let _ = file.set_len( len - amount as u64);
                // file has been changed, check if other clients need to be informed
                monitor_manager.inform_monitors(path, self.read_file(&mut file));
            }
        } else {
            // offload monitor requests to the monitor manager
            let interval: u32 = self.read_int();
            println!("{style_bold}{color_magenta}[RequestHandler]:{style_reset} client has requested to monitor {path_str} for {interval}ms");
            monitor_manager.add_interval(path, addr, interval)
        }

        // no errors encountered
        response.status = StatusCode::GOOD;
        return response;
    }

    // this function is called for every request that is received
    fn process_request(&mut self, dir: &PathBuf, monitor_manager: &mut MonitorManager, addr: SocketAddr, response_manager: &mut ResponseManager, at_most_once: bool) -> ResponseMarshal {
        // all requests have a request number and an operation type in a byte
        let req_no = self.read_int();
        let op: u8 = self.read_byte();

        // handle handshakes here
        if op == RequestOperation::HANDSHAKE {
            println!("{style_bold}{color_magenta}[RequestHandler]:{style_reset} client initiated handshake");
            let client_time = self.read_int();
            if response_manager.session_map.contains_key(&addr) {
                // the client already exists in the session map
                println!("{style_bold}{color_blue}[ResponseManager]:{style_reset} Found key for {client_time}");
                if response_manager.session_map.get(&addr).unwrap() != &client_time {
                    /* if the client_time is not the same, perhaps the client exited without disconnecting last time
                       which means the data associated with the client might not have been flushed */
                    println!("{style_bold}{color_blue}[ResponseManager]:{style_reset} Clearing old responses from this client. Previous session was {client_time}");
                    response_manager.flush_client(&addr);
                    response_manager.session_map.insert(addr, client_time);
                }
                // if the client_time is the same then do nothing, most likely a repeat request
            } else {
                // if the client doesnt exist in the session map then we add an entry
                println!("{style_bold}{color_blue}[ResponseManager]:{style_reset} Creating blank response map for this client. Session id is {client_time}");
                response_manager.session_map.insert(addr, client_time);
            }
            println!("{style_bold}{color_blue}[ResponseManager]:{style_reset} Sending handshake confirmation");
            return ResponseMarshal{status: StatusCode::GOOD, data: "Handshake Completed. Welcome!".to_owned()};
        }

        // handle disconnects
        if op == RequestOperation::DISCONNECT {
            // simply flush any saved client data and return a message
            println!("{style_bold}{color_magenta}[RequestHandler]:{style_reset} client wants to disconnect");
            response_manager.flush_client(&addr);
            println!("{style_bold}{color_blue}[ResponseManager]:{style_reset} Sending disconnect confirmation");
            return ResponseMarshal{status: StatusCode::GOOD, data: "Bye!".to_owned()};
        }

        if at_most_once {
            // duplicate filtering for at-most-once semantics
            if response_manager.has_entry(&addr, req_no) {
                // return saved response if it exists
                println!("{style_bold}{color_blue}[ResponseManager]:{style_reset} Req No. {req_no} from client is a duplicate! Sending back saved response.");
                return response_manager.get_entry(&addr, req_no).clone(); 
            } else {
                println!("{style_bold}{color_blue}[ResponseManager]:{style_reset} No saved response found for req no. {req_no} from client");
            }
        }

        // call the parse request function to service user requests
        let response = self.parse_request(op, dir, monitor_manager, addr);
        let data = &response.data;
        let mut status = "GOOD";
        if response.status == StatusCode::BAD {
            status = "BAD";
        }
        println!("{style_bold}{color_blue}[ResponseManager]:{style_reset} Response has status code {status} with data '{data}'");
        if at_most_once {
            // save response if using at-most-once semantics
            response_manager.add_entry(&addr, req_no, response.clone());
        }

        // return response to be sent back to the client
        let response_data = &response.data;
        println!("{style_bold}{color_blue}[ResponseManager]:{style_reset} Sending data '{response_data}'");
        return response;
        
    }
}

// all packets are sent through this function
fn send(socket: &UdpSocket, response: &ResponseMarshal, addr: SocketAddr) {
    let result = socket.send_to(&response.into_bytes(), addr);
    if result.is_ok() {
        let amt = result.unwrap();
        println!("{style_bold}{color_green}[UDP]:{style_reset} Sent {amt} bytes to {addr}");
    } else {
        let err: Error = result.unwrap_err();
        println!("{style_bold}{color_green}[UDP]:{style_reset} Error Sending Data: {err}");
    }

}

fn main() {
    // gracefully handle a ctrl-c event as a way to close the server
    let _ = ctrlc::set_handler(move || { 
        println!("{style_bold}{color_yellow}[Server]:{style_reset} Closing"); 
        process::exit(0);
    });

    // parse cli arguments
    let args: Args = Args::parse();
    let dir = &args.dir;
    let mut path = Path::new(dir).to_path_buf();
    if dir.len() == 0 { // use home directory when no server file directory is specified
        path = dirs::home_dir().unwrap();
    }

    // the server file directory must exist so we exit with an error code of 1 if it doesn't
    if !(path.is_dir() && path.exists())  {
        println!("{style_bold}{color_yellow}[Server]:{style_reset} Couldn't find server file directory!");
        process::exit(1);
    }

    // bind a socket for the server 
    let server_address:&str = &(args.server_host + ":" + &args.port.to_string());
    let socket: UdpSocket = UdpSocket::bind(server_address).expect(&format!("Couldn't bind to address {server_address}"));

    // Print all the command line arguments for verification
    println!("{style_bold}{color_yellow}[Server]:{style_reset} Server bound to {server_address}");
    let dir_str = path.to_string_lossy();
    println!("{style_bold}{color_yellow}[Server]:{style_reset} Server file directory is {dir_str}");
    if args.at_most_once {
        println!("{style_bold}{color_yellow}[Server]:{style_reset} Using at-most-once semantics");
    } else {
        println!("{style_bold}{color_yellow}[Server]:{style_reset} Using at-least-once semantics");
    }

    // initialize the monitor and response managers to be used for this run
    let mut monitor_manager: MonitorManager<'_> = MonitorManager{dict: HashMap::new(), socket: &socket};
    let mut response_manager: ResponseManager = ResponseManager{response_map: HashMap::new(), session_map: HashMap::new()};

    loop { // while(True)
        // keep listening for data on the socket
        let mut buf: [u8; 1048576] = [0; 1024*1024];
        let result: Result<(usize, SocketAddr), Error> = socket.recv_from(&mut buf);
        if result.is_ok() {
            // if no errors reading the data, process the request and send back a reply
            let (amt, src) = result.unwrap();
            println!("{style_bold}\n{color_green}[UDP]:{style_reset} Received {amt} bytes from {src}");
            let mut handler : RequestHandler = RequestHandler{buf: &buf, i: 0};
            let response = handler.process_request(&path, &mut monitor_manager, src, &mut response_manager, args.at_most_once);
            send(&socket, &response, src);
        } else {
            let err: Error = result.unwrap_err();
            println!("{style_bold}{color_green}[UDP]:{style_reset} Error Receiving Data: {err}");
        }
    }
}
