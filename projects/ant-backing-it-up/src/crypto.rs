use aes_gcm::{
    aead::{Aead, OsRng},
    AeadCore, Aes256Gcm, KeyInit,
};
use sha2::digest::generic_array::GenericArray;

pub fn decrypt_backup(nonce: &Vec<u8>, ciphertext: &Vec<u8>) -> Vec<u8> {
    let key = ant_library::secret::load_secret_binary("ant_backing_it_up_key")
        .expect("ant_backing_it_up_key secret");
    let cipher = Aes256Gcm::new(&GenericArray::clone_from_slice(&key[0..32])); // 256 bits = 8 * 32 bytes

    let plaintext = cipher
        .decrypt(
            &GenericArray::clone_from_slice(&nonce),
            ciphertext.as_slice(),
        )
        .expect("decrypt");

    plaintext
}

pub struct Encryption {
    pub ciphertext: Vec<u8>,
    pub nonce: Vec<u8>,
}

pub fn encrypt_backup(plaintext: &Vec<u8>) -> Encryption {
    let key = ant_library::secret::load_secret_binary("ant_backing_it_up_key")
        .expect("ant_backing_it_up_key secret");

    let cipher = Aes256Gcm::new(&GenericArray::clone_from_slice(&key[0..32])); // 256 bits = 8 * 32 bytes
    let nonce = Aes256Gcm::generate_nonce(&mut OsRng); // Unique per file

    let ciphertext = cipher.encrypt(&nonce, plaintext.as_ref()).unwrap();

    Encryption {
        ciphertext,
        nonce: nonce.to_vec(),
    }
}
