use chess_networking::*;
use std::io::prelude::*;
use std::net::{TcpListener, TcpStream};

pub trait ChessProtocol {
    fn set_blocking(&mut self, block: bool) -> std::io::Result<()>;
    fn handle_setup(&mut self, desired_start: Start) -> std::io::Result<Start>;
    fn send_move(&mut self, m: Move) -> std::io::Result<()>;
    fn receive_move(&mut self) -> std::io::Result<Option<Move>>;
    fn receive_ack(&mut self) -> std::io::Result<Option<Ack>>;
    fn send_ack(&mut self, ack: Ack) -> std::io::Result<()>;
}

pub struct Server {
    listener: TcpListener,
    stream: TcpStream,
}

impl Server {
    pub fn new(address: &str) -> std::io::Result<Server> {
        let listener = TcpListener::bind(address)?;
        let stream = listener.accept()?.0;
        Ok(Server { listener, stream })
    }
}

impl ChessProtocol for Server {
    fn set_blocking(&mut self, block: bool) -> std::io::Result<()> {
        self.stream.set_nonblocking(!block)?;
        Ok(())
    }

    fn send_ack(&mut self, ack: Ack) -> std::io::Result<()> {
        let bytes: Vec<u8> = ack.try_into().unwrap();
        self.stream.write(&bytes)?;

        Ok(())
    }

    fn receive_ack(&mut self) -> std::io::Result<Option<Ack>> {
        let mut buf: [u8; 1024] = [0; 1024];
        let length = match self.stream.read(&mut buf) {
            Ok(l) => l,
            Err(_) => return Ok(None),
        };

        let ack: Ack = buf[0..length].try_into().unwrap();
        Ok(Some(ack))
    }

    fn handle_setup(&mut self, mut desired_start: Start) -> std::io::Result<Start> {
        let mut buf: [u8; 1024] = [0; 1024];
        let length = self.stream.read(&mut buf)?;
        let what_client_wants: Start = buf[0..length].try_into().unwrap();

        let mut client = desired_start.clone();
        client.is_white = !desired_start.is_white;

        let bytes: Vec<u8> = client.try_into().unwrap();
        self.stream.write(&bytes)?;

        Ok(desired_start)
    }

    fn send_move(&mut self, m: Move) -> std::io::Result<()> {
        let bytes: Vec<u8> = m.try_into().unwrap();
        self.stream.write(&bytes)?;

        Ok(())
    }

    fn receive_move(&mut self) -> std::io::Result<Option<Move>> {
        let mut buf: [u8; 1024] = [0; 1024];
        let length = match self.stream.read(&mut buf) {
            Ok(l) => l,
            Err(_) => return Ok(None),
        };

        let m: Move = buf[0..length].try_into().unwrap();
        Ok(Some(m))
    }
}

pub struct Client {
    stream: TcpStream,
}

impl Client {
    pub fn new(address: &str) -> std::io::Result<Client> {
        let stream = TcpStream::connect(address)?;
        Ok(Client { stream })
    }
}

impl ChessProtocol for Client {
    fn set_blocking(&mut self, block: bool) -> std::io::Result<()> {
        self.stream.set_nonblocking(!block)?;
        Ok(())
    }

    fn send_ack(&mut self, ack: Ack) -> std::io::Result<()> {
        let bytes: Vec<u8> = ack.try_into().unwrap();
        self.stream.write(&bytes)?;

        Ok(())
    }

    fn receive_ack(&mut self) -> std::io::Result<Option<Ack>> {
        let mut buf: [u8; 1024] = [0; 1024];
        let length = match self.stream.read(&mut buf) {
            Ok(l) => l,
            Err(_) => return Ok(None),
        };

        let ack: Ack = buf[0..length].try_into().unwrap();
        Ok(Some(ack))
    }

    fn handle_setup(&mut self, desired_start: Start) -> std::io::Result<Start> {
        let bytes: Vec<u8> = desired_start.try_into().unwrap();
        self.stream.write(&bytes)?;

        let mut buf: [u8; 1024] = [0; 1024];
        let length = self.stream.read(&mut buf)?;
        let actual_start: Start = buf[0..length].try_into().unwrap();

        Ok(actual_start)
    }

    fn send_move(&mut self, m: Move) -> std::io::Result<()> {
        let bytes: Vec<u8> = m.try_into().unwrap();
        self.stream.write(&bytes)?;

        Ok(())
    }

    fn receive_move(&mut self) -> std::io::Result<Option<Move>> {
        let mut buf: [u8; 1024] = [0; 1024];
        let length = match self.stream.read(&mut buf) {
            Ok(l) => l,
            Err(_) => return Ok(None),
        };

        let m: Move = buf[0..length].try_into().unwrap();
        Ok(Some(m))
    }
}
