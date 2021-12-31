use std::env;
use std::process::Command;

use anyhow::Result;

use super::{ExecuteContext, Task};
use crate::MinioConfig;

const HUMMOCK_REMOTE_NAME: &str = "hummock-minio";

pub struct ConfigureMinioTask {
    mcli_path: String,
    mcli_config_path: String,
    config: MinioConfig,
}

impl ConfigureMinioTask {
    pub fn new(config: MinioConfig) -> Result<Self> {
        let prefix_bin = env::var("PREFIX_BIN")?;
        let prefix_config = env::var("PREFIX_CONFIG")?;
        Ok(Self {
            mcli_path: format!("{}/mcli", prefix_bin),
            mcli_config_path: format!("{}/mcli", prefix_config),
            config,
        })
    }

    fn mcli(&mut self) -> Command {
        let mut cmd = Command::new(self.mcli_path.clone());
        cmd.arg("-C").arg(&self.mcli_config_path);
        cmd
    }
}

impl Task for ConfigureMinioTask {
    fn execute(&mut self, ctx: &mut ExecuteContext<impl std::io::Write>) -> anyhow::Result<()> {
        ctx.pb.set_message("waiting for online...");
        let minio_address = format!("{}:{}", self.config.address, self.config.port);
        let minio_console_address = format!(
            "{}:{}",
            self.config.console_address, self.config.console_port
        );
        ctx.wait_tcp(&minio_address)?;

        ctx.pb.set_message("configure...");

        let mut cmd = self.mcli();
        cmd.arg("alias")
            .arg("set")
            .arg(HUMMOCK_REMOTE_NAME)
            .arg(format!("http://{}", minio_address))
            .arg(env::var("MINIO_ROOT_USER")?)
            .arg(env::var("MINIO_ROOT_PASSWORD")?);

        ctx.run_command(cmd)?;

        let mut cmd = self.mcli();
        cmd.arg("admin")
            .arg("user")
            .arg("add")
            .arg(format!("{}/", HUMMOCK_REMOTE_NAME))
            .arg(env::var("MINIO_HUMMOCK_USER")?)
            .arg(env::var("MINIO_HUMMOCK_PASSWORD")?);
        ctx.run_command(cmd)?;

        let mut cmd = self.mcli();
        cmd.arg("admin")
            .arg("policy")
            .arg("set")
            .arg(format!("{}/", HUMMOCK_REMOTE_NAME))
            .arg("readwrite")
            .arg(format!("user={}", env::var("MINIO_HUMMOCK_USER")?));
        ctx.run_command(cmd)?;

        let mut cmd = self.mcli();
        cmd.arg("mb").arg(format!(
            "{}/{}",
            HUMMOCK_REMOTE_NAME,
            env::var("MINIO_BUCKET_NAME")?
        ));
        ctx.run_command(cmd).ok();

        ctx.complete_spin();

        ctx.pb.set_message(format!(
            "api http://{}/, console http://{}/",
            minio_address, minio_console_address
        ));

        Ok(())
    }
}
