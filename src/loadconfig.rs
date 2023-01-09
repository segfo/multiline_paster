use std::marker::PhantomData;
use serde_derive::{Serialize,Deserialize};
use serde::de::DeserializeOwned;

pub struct TomlConfigDeserializer<T>{
    _t:PhantomData<T>
}

use std::fs::OpenOptions;
use std::io::{BufReader,prelude::*};

impl<T> TomlConfigDeserializer<T> where T:DeserializeOwned{
    pub fn from_file(filepath:&str)->Result<T,Box<dyn std::error::Error>>{
        let f = OpenOptions::new()
        .read(true).write(false).create_new(false)
        .open(filepath)?;
        TomlConfigDeserializer::from_reader(f)
    }

    pub fn from_reader<R>(rdr:R)->Result<T,Box<dyn std::error::Error>> where R:Read{
        let mut br = BufReader::new(rdr);
        let mut buf = String::new();
        br.read_to_string(&mut buf);
        let t = toml::from_str(&buf)?;
        Ok(t)
    }
}
