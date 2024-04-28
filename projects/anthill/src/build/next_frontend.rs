use command_macros::cmd;

pub async fn build() {
    cmd!(next build).status().unwrap();
}
