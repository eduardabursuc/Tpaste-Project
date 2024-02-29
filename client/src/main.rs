use std::fs::{File, OpenOptions};
use std::io::{self, BufRead, BufReader, Read, Write};
use std::net::TcpStream;
use std::str;
use std::sync::{Arc, Mutex};

fn main() -> io::Result<()> {
    let mut terminal_output = String::new();
    io::stdin()
        .read_to_string(&mut terminal_output)
        .expect("Failed to read from stdin");

    let mut stream = TcpStream::connect("127.0.0.1:7878")?;

    let mut token: String = String::from("not a token");

    let exe_path = std::env::current_exe()?;
    let token_file_path = exe_path.with_file_name("token.txt");

    if !token_file_path.exists() {
        File::create(&token_file_path)?;
    }

    let file = OpenOptions::new()
        .read(true)
        .write(true)
        .open(&token_file_path)?;

    let reader = BufReader::new(&file);

    if let Some(Ok(tokn)) = reader.lines().next() {
        token = tokn.trim().to_string();
    }

    stream.write_all(token.as_bytes())?;

    let mut buffer: Vec<u8> = Vec::new();
    let mut reader = BufReader::new(&stream);
    reader.read_until(b'\n', &mut buffer)?;

    let mut received_message = str::from_utf8(&buffer).unwrap().trim();
    let mut write_stream = stream.try_clone()?;

    if received_message != "valid" {
        let file = OpenOptions::new().read(true).write(true).open("CONIN$")?;
        let mut buf_reader = BufReader::new(file);
        let mut input = String::new();

        while !received_message.starts_with("Token:") {
            println!("{}", received_message);
            input.clear();
            buf_reader
                .read_line(&mut input)
                .expect("Failed to read from terminal.");
            write_stream.write_all(input.as_bytes())?;
            buffer.clear();
            reader.read_until(b'\n', &mut buffer)?;
            received_message = str::from_utf8(&buffer).unwrap().trim();
        }

        let received_token = received_message.trim_start_matches("Token: ");

        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .open(&token_file_path)?;

        let file_mutex = Arc::new(Mutex::new(file));
        let mut file_lock = file_mutex.lock().unwrap();

        file_lock.write_all(received_token.as_bytes())?;
    }

    write_stream.write_all(terminal_output.as_bytes())?;
    buffer.clear();
    reader.read_until(b'\n', &mut buffer)?;
    received_message = str::from_utf8(&buffer).unwrap().trim();
    println!("{}", received_message);

    Ok(())
}
