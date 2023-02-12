extern crate regex;
extern crate bufstream;

use self::regex::Regex;
use std::net::{TcpListener, TcpStream};
use bufstream::BufStream;
use std::thread;
use std::fs::File;
use std::io::prelude::*;
use std::sync::{Arc, Mutex};
use std::str;
use std::path::Path;

const DEFAULT_ADDRESS: &'static str = "0.0.0.0:8000";
const DEFAULT_WORKERS: u32 = 4;
const DEFAULT_REQUEST_SIZE: usize = 1024;

fn main() {
    //vars to be loaded from conf
    let address = DEFAULT_ADDRESS;
    let worker_no = DEFAULT_WORKERS;

    let listener = match TcpListener::bind(address){
        Ok(acc) => acc,
        Err(e) => panic!("{}", e),
    };

    println!("httpd_rs 0.1.0\nhttp://{}", address);

    //vec of the incoming streams
    let streams:Vec<TcpStream> = Vec::new();
    let data = Arc::new(Mutex::new(streams));

    //spawn some worker threads
    for _ in 0..worker_no {
        let data = data.clone();
        thread::spawn(move || {
            loop {
                let mut data = match data.lock() {
                    Ok(data) => data,
                    Err(e) => {
                        println!("{}", e);
                        break;
                    },
                };

                let _ = match data.pop() {
                    Some(s) => {
                        let mut stream = BufStream::new(s);
                        handle_client(&mut stream)
                    },
                    None => (),
                };
            }
        });
    }

    //loop for accepting requests
    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                let data = data.clone();
                let mut data = match data.lock() {
                    Ok(data) => data,
                    Err(e) => {
                        println!("{}", e);
                        break;
                    },
                };
                data.push(stream);
            },
            Err(e) => println!("error: {}", e),
        }
    }

    drop(listener);
}


fn get_route(request:String) -> String {
    let req_re = match Regex::new("(?P<type>[A-Z^']+) (?P<route>[^']+) HTTP/(?P<http>[^']+)"){
        Ok(re) => re,
        Err(e) => {
            println!("error: {}", e);
            return "/".to_string();
        },
    };

    let caps = match req_re.captures(request.as_ref()) {
        Some(c) => c,
        None => {
            println!("error: no captures");
            return "/".to_string();
        },
    };

    let full_path: &str = match caps.name("route") {
        Some(c) => c.as_str(),
        None => {
            println!("error: no route capture");
            return "/".to_string();
        },
    };

    let split_path: Vec<&str> = full_path.split('?').collect();

    split_path[0].to_string()
}

fn get_mimetype(path:&str) -> String {
    match path {
        //text
        "html" => "text/html; charset=utf8",
        "css" => "text/css",
        "csv" => "text/csv",
        "rtf" => "text/rtf",
        //application
        "js" => "application/javascript",
        "json" => "application/json",
        "xml" => "application/xml",
        //images
        "png" => "image/png",
        "jpeg" | "jpg" => "image/jpg",
        "gif" => "image/gif",
        "bmp" => "image/bmp",
        "svg" => "image/svg+xml",
        "tiff" => "image/tiff",
        //default
        _ => "text/plain",
    }.to_string()
}

fn handle_client(stream: &mut BufStream<TcpStream>) { 
    let mut byte_req: [u8; DEFAULT_REQUEST_SIZE] = [0; DEFAULT_REQUEST_SIZE];
    let _ = match stream.read(&mut byte_req) {
        Ok(_) => (),
        Err(e) => {
            println!("error: {}", e);
            return;
        },
    };

    let request:String = match  str::from_utf8(&byte_req){
        Ok(req) => req.to_string(),
        Err(e) => {
            println!("error: {}", e);
            return;
        },
    };

    let mut route:String = get_route(request);

    if route == "/" {
        route = "/index.html".to_string();
    }

    println!("GET: {}", route);

    //if the file is not found need to 404
    let path_str = format!("/httpd_rd_root{}",route);
    let path = Path::new(&path_str);
    let response = match File::open(path) {
        Ok(f) => {
            let mut file = f;

            let mut file_buf:Vec<u8> = Vec::new();
            let _ = file.read_to_end(&mut file_buf);

            let mimetype = get_mimetype(path.extension().unwrap().to_str().unwrap());

            let headers = format!("HTTP/1.1 200 OK\r\nContent-Type: {}\r\ncontent-length: {}\r\n\r\n", mimetype, file_buf.len());

            let mut res:Vec<u8> = headers.into_bytes();
            res.append(&mut file_buf);
            
            res
        },
        Err(e) => {
            println!("file open error: {}", e);
            let header = format!("HTTP/1.1 404 NOT FOUND\r\n\r\n");
            header.into_bytes()
        },
    };

    match stream.write_all(response.as_slice()){
        Ok(_) => return,
        Err(e) => {
            println!("response write error: {}", e);
            return;
        },
    }       
}
