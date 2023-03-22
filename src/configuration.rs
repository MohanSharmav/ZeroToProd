
#[derive(serde::Deserialize)]
pub struct Settings{
    pub database: DatabaseSettings,
    pub application: ApplicationSettings,
}

#[derive(serde::Deserialize)]
pub struct DatabaseSettings{
    pub username: String,
    pub password: String,
//pub password: Secret<String>,
    pub port: u16,
    pub host: String,
    pub database_name: String,
}

#[derive(serde::Deserialize)]
pub struct ApplicationSettings{
    pub port:u16,
    pub host: String,
}

pub fn get_configuration() -> Result<Settings, config::ConfigError> {
    let base_path = std::env::current_dir()
        .expect("failed to get current directory");
    let configuration_directory = base_path.join("configuration");

    let environment: Environment = std::env::var("APP_ENVIRONMENT")
        .unwrap_or_else(|_| "local".into())
        .try_into()
        .expect("failed to parse App_Environment");

    let environment_filename = format!("{}.yaml", environment.as_str());

    let settings = config::Config::builder()
        .add_source(
            config::File::from(configuration_directory.join("base.yaml"))
        )
        .add_source(
            config::File::from(configuration_directory.join(environment_filename))
        )
        .add_source(
            config::Environment::with_prefix("APP")
                .prefix_separator("_")
                .separator("__")
        )
        .build()?;
    settings.try_deserialize::<Settings>()
}

    //
    // let settings = config::Config::builder()
    //     .add_source(config::File::new("configuration.yaml",config::FileFormat::Yaml))
    //     .build()?;
    // settings.try_deserialize::<Settings>()
// }

pub enum Environment{
    Local,
    Production,
}

impl Environment {
    pub fn as_str(&self) -> &'static str {
        match self{
            Environment::Local=>"local",
            Environment::Production=>"production",
        }
    }
}
impl TryFrom<String> for Environment {
    type Error = String;
    fn try_from(s: String) -> Result<Self, Self::Error> {
        match s.to_lowercase().as_str() {
        "local" => Ok(Self::Local),
        "production" => Ok(Self::Production),
        other => Err(format!(
            "{} is not a supported environment. \
                Use either `local` or `production`.",
            other
        )),
    }
    }
}
//
// impl TryFrom<String> for Environment {
// type Error =String ;
//
// fn try_from(s: String) -> Result<Self,Self::Error>{
//     match s.to_lowercase().as_str(){
//         "local" => Ok(Self::Local),
//         "production" => Ok(Self::Production),
//         other => Err(format!(
//             "{} is not supported environment .\
//             use either 'local' or 'production'",other
//         )),
//
//             )
//     }
// }
// }

impl DatabaseSettings{
    pub fn connection_string(&self) -> String {
        format!(
            "postgres://{}:{}@{}:{}/{}",
            self.username,self.password,self.host,self.port,self.database_name
        )
    }

    pub fn connection_string_without_db(&self) -> String{
        format!(
            "postgres://{}:{}@{}:{}",
            self.username,self.password,self.host,self.port
        )
    }
}