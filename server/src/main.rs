use chrono::{DateTime, Local};
use jsonwebtoken::{decode, encode, Algorithm, DecodingKey, EncodingKey, Header, Validation};
use rand::distributions::Alphanumeric;
use rand::{thread_rng, Rng};
use rusqlite::{params, Connection, Result};
use serde::{Deserialize, Serialize};
use std::fs::{File, OpenOptions};
use std::io::{self, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, SystemTime};

#[derive(Debug, Serialize, Deserialize)]
struct UserClaims {
    sub: String,
    exp: usize,
}

fn create_users_table() -> Result<()> {
    let conn = Connection::open("users.db")?;

    conn.execute(
        "create table if not exists users (
            username text primary key,
            password text not null
        )",
        (),
    )?;

    Ok(())
}

fn generate_token(username: &str) -> String {
    let expiration = SystemTime::now() + Duration::from_secs(60 * 24 * 60 * 60);

    let claims = UserClaims {
        sub: username.to_owned(),
        exp: expiration
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_secs() as usize,
    };

    let secret: &[u8] = b"secret_key";
    let secret_key = EncodingKey::from_secret(secret);

    encode(&Header::default(), &claims, &secret_key).unwrap()
}

fn validate_token(token: &str) -> Result<UserClaims, jsonwebtoken::errors::Error> {
    let secret: &[u8] = b"secret_key";
    let secret_key = DecodingKey::from_secret(secret);

    decode::<UserClaims>(token, &secret_key, &Validation::new(Algorithm::HS256))
        .map(|data| data.claims)
}

fn registration(
    username: &str,
    password: &str,
    mut stream: TcpStream,
) -> Result<(), rusqlite::Error> {
    let conn = Connection::open("users.db")?;

    let query = "SELECT COUNT(*) FROM users WHERE username = ?1";
    let count: i64 = conn.query_row(query, params![username], |row| row.get(0))?;

    if count > 0 {
        stream
            .write_all(b"User exists.\n")
            .expect("Error at writing to the stream.");
        return Err(rusqlite::Error::QueryReturnedNoRows);
    } else {
        let token = generate_token(username);
        let clone = token.clone();
        let insert_query = "INSERT INTO users (username, password) VALUES (?1, ?2)";

        let db_mutex = Arc::new(Mutex::new(conn));
        let db_lock = db_mutex.lock().unwrap();

        if let Err(err) = db_lock.execute(insert_query, params![username, password]) {
            eprintln!("Error executing query: {:?}", err);
            println!("Registration failed.");
            return Err(err);
        } else {
            stream
                .write_all(format!("Token: {clone}\n").as_bytes())
                .expect("Error at writing to the stream.");
        }
    }

    Ok(())
}

fn login(username: &str, password: &str, mut stream: TcpStream) -> Result<(), rusqlite::Error> {
    let conn = Connection::open("users.db")?;

    let stored_password: Result<String> = conn.query_row(
        "SELECT password FROM users WHERE username = ?1",
        params![username],
        |row| row.get(0),
    );

    match stored_password {
        Ok(passwd) => {
            if passwd == password {
                let token = generate_token(username);
                let clone = token.clone();

                stream
                    .write_all(format!("Token: {clone}\n").as_bytes())
                    .expect("Error at writing to the stream.");

                Ok(())
            } else {
                stream
                    .write_all(b"Incorrect password.\n")
                    .expect("Error at writing to the stream.");
                Err(rusqlite::Error::QueryReturnedNoRows)
            }
        }
        Err(e) => {
            stream
                .write_all(b"User not found.\n")
                .expect("Error at writing to the stream.");
            Err(e)
        }
    }
}

fn upload_to_file(path: &str, content: String) -> Result<(), io::Error> {
    let file_path: PathBuf = PathBuf::from(path);

    if !file_path.exists() {
        File::create(&file_path)?;
    }

    let file = OpenOptions::new()
        .write(true)
        .append(true)
        .open(&file_path)?;

    let file_mutex = Arc::new(Mutex::new(file));
    let mut file_lock = file_mutex.lock().unwrap();

    file_lock.write_all(content.as_bytes())?;

    Ok(())
}

fn link_generator() -> String {
    let rng = thread_rng();
    let random = rng.sample_iter(&Alphanumeric).take(8).collect();
    format!(
        "http://127.0.0.1:3030/tpaste/{}",
        String::from_utf8(random).expect("Failed to convert random string.")
    )
}

fn add_timestamp(content: String) -> String {
    let timestamp: DateTime<Local> = Local::now();
    let formatted_timestamp = timestamp.format("%Y-%m-%d %H:%M").to_string();
    format!("{}{}\n\n", content, formatted_timestamp).to_string()
}

fn handle_user(mut stream: TcpStream) -> io::Result<()> {
    let mut user = String::new();

    let mut token: [u8; 256] = [0; 256];
    let bytes_read = stream.read(&mut token)?;

    if bytes_read > 0 {
        let token_str = String::from_utf8_lossy(&token[..bytes_read]);
        if !token_str.trim().is_empty() {
            if let Ok(claims) = validate_token(&token_str) {
                user = claims.sub;
                println!("User: {}", user);
                stream.write_all(b"valid\n")?;
            } else {
                stream.write_all(b"Authentication required:\n")?;
            }
        }
    }

    while user.is_empty() {
        let mut buf: [u8; 10001] = [0; 10001];
        let bytes_read = stream.read(&mut buf)?;

        let command = String::from_utf8_lossy(&buf[..bytes_read]);
        println!("{command}");
        let parts: Vec<&str> = command.split_whitespace().collect();

        match parts.as_slice() {
            ["login", username, password] => {
                if !user.is_empty() {
                    stream.write_all(b"Already logged.\n")?;
                } else {
                    println!("Received LOGIN command for user: {}", username);
                    if let Ok(()) = login(username, password, stream.try_clone()?) {
                        user = username.to_string();
                    }
                }
            }
            ["registration", username, password] => {
                if !user.is_empty() {
                    stream.write_all(b"Already logged.\n")?;
                } else {
                    println!("Received REGISTRATION command for user: {}", username);
                    if let Ok(()) = registration(username, password, stream.try_clone()?) {
                        user = username.to_string();
                    }
                }
            }
            _ => {
                stream.write_all(b"Wrong syntax.\n")?;
            }
        }
    }

    let mut buf: [u8; 10001] = [0; 10001];
    let bytes_read = stream.read(&mut buf)?;
    let output = String::from_utf8_lossy(&buf[..bytes_read]);

    let content = output.to_string();
    let link = link_generator();

    let destination = link.trim_start_matches("http://127.0.0.1:3030/tpaste/");

    match upload_to_file(
        format!("./data/{}.txt", destination).as_str(),
        content.clone(),
    ) {
        Ok(_) => {}
        Err(e) => {
            eprintln!("Error: {e}")
        }
    }

    match upload_to_file(
        format!("./pastes/{}.txt", user).as_str(),
        add_timestamp(format!("{}\n", link.clone())),
    ) {
        Ok(_) => {}
        Err(e) => {
            eprintln!("Error: {e}")
        }
    }

    stream
        .write_all(link.as_bytes())
        .expect("Failed writing to stream.");

    Ok(())
}

fn main() -> io::Result<()> {
    create_users_table().expect("Error at creating data base table.");

    let receiver_listener =
        TcpListener::bind("127.0.0.1:7878").expect("Failed and bind with the sender");

    let mut thread_vec: Vec<thread::JoinHandle<()>> = Vec::new();

    for stream in receiver_listener.incoming() {
        let stream = stream.expect("failed");

        let handle = thread::spawn(move || {
            handle_user(stream).unwrap_or_else(|error| eprintln!("{:?}", error))
        });

        thread_vec.push(handle);
    }

    for handle in thread_vec {
        handle.join().unwrap();
    }

    Ok(())
}
