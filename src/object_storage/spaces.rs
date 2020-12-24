use std::fs::{read_to_string, File};
use std::path::Path;

use chrono::Utc;
use retry::delay::Fibonacci;
use retry::{Error, OperationResult};
use rusoto_core::{Client, HttpClient, Region};
use rusoto_credential::StaticProvider;
use rusoto_s3::{GetObjectRequest, S3Client, S3};
use tokio::io;

use crate::error::{cast_simple_error_to_engine_error, EngineError, EngineErrorCause};
use crate::models::{Context, StringPath};
use crate::object_storage::{Kind, ObjectStorage};
use crate::runtime;

pub struct Spaces {
    context: Context,
    id: String,
    name: String,
    access_key_id: String,
    secret_access_key: String,
    region: String,
}

impl Spaces {
    pub fn new(
        context: Context,
        id: String,
        name: String,
        access_key_id: String,
        secret_access_key: String,
        region: String,
    ) -> Self {
        Spaces {
            context,
            id,
            name,
            access_key_id,
            secret_access_key,
            region,
        }
    }

    async fn get_object<T, S, X>(
        &self,
        bucket_name: T,
        object_key: S,
        download_into_file_path: X,
    ) -> Result<File, EngineError>
    where
        T: Into<String>,
        S: Into<String>,
        X: AsRef<Path>,
    {
        // Digital ocean doesn't implement any space download, it use the generic AWS SDK
        let region = Region::Custom {
            name: self.region.clone(),
            endpoint: format!("https://{}.digitaloceanspaces.com", self.region),
        };

        let credentials = StaticProvider::new(
            self.access_key_id.clone(),
            self.secret_access_key.clone(),
            None,
            None,
        );

        let client = Client::new_with(credentials, HttpClient::new().unwrap());
        let s3_client = S3Client::new_with_client(client, region.clone());
        let object = s3_client
            .get_object(GetObjectRequest {
                bucket: bucket_name.into(),
                key: object_key.into(),
                ..Default::default()
            })
            .await;

        match object {
            Ok(mut obj_bod) => {
                let body = obj_bod.body.take();
                let mut body = body.unwrap().into_async_read();
                let file = tokio::fs::File::create(download_into_file_path.as_ref()).await;
                match file {
                    Ok(mut created_file) => match io::copy(&mut body, &mut created_file).await {
                        Ok(_) => Ok(File::open(download_into_file_path.as_ref()).unwrap()),
                        Err(e) => {
                            Err(self.engine_error(EngineErrorCause::Internal, format!("{:?}", e)))
                        }
                    },
                    Err(e) => {
                        Err(self.engine_error(EngineErrorCause::Internal, format!("{:?}", e)))
                    }
                }
            }
            Err(e) => Err(self.engine_error(EngineErrorCause::Internal, format!("{:?}", e))),
        }
    }
}

impl ObjectStorage for Spaces {
    fn context(&self) -> &Context {
        &self.context
    }

    fn kind(&self) -> Kind {
        Kind::Spaces
    }

    fn id(&self) -> &str {
        self.id.as_str()
    }

    fn name(&self) -> &str {
        self.name.as_str()
    }

    fn is_valid(&self) -> Result<(), EngineError> {
        // TODO check valid credentials
        Ok(())
    }

    fn create_bucket<S>(&self, bucket_name: S) -> Result<(), EngineError>
    where
        S: Into<String>,
    {
        unimplemented!()
    }

    fn delete_bucket<S>(&self, bucket_name: S) -> Result<(), EngineError>
    where
        S: Into<String>,
    {
        unimplemented!()
    }

    fn get<T, S>(&self, bucket_name: T, object_key: S) -> Result<(StringPath, File), EngineError>
    where
        T: Into<String>,
        S: Into<String>,
    {
        let bucket_name = bucket_name.into();
        let object_key = object_key.into();

        let workspace_directory = crate::fs::workspace_directory(
            self.context().workspace_root_dir(),
            self.context().execution_id(),
            format!("object-storage/s3/{}", self.name()),
        );

        let file_path = format!(
            "{}/{}/{}",
            workspace_directory,
            bucket_name.as_str(),
            object_key.as_str()
        );

        let result = retry::retry(
            Fibonacci::from_millis(3000).take(5),
            || match runtime::async_run(self.get_object(
                bucket_name.as_str(),
                object_key.as_str(),
                file_path.as_str(),
            )) {
                Ok(file) => OperationResult::Ok(file),
                Err(err) => {
                    debug!("{:?}", err);

                    warn!(
                        "Can't download object '{}'. Let's retry...",
                        object_key.as_str()
                    );

                    OperationResult::Retry(err)
                }
            },
        );

        let file = match result {
            Ok(_) => File::open(file_path.as_str()),
            Err(err) => {
                return match err {
                    Error::Operation { error, .. } => Err(error),
                    Error::Internal(err) => Err(self.engine_error(EngineErrorCause::Internal, err)),
                };
            }
        };

        match file {
            Ok(file) => Ok((file_path, file)),
            Err(err) => Err(self.engine_error(EngineErrorCause::Internal, format!("{:?}", err))),
        }
    }
}

// TODO to cleanup
pub async fn download_space_object(
    access_key_id: &str,
    secret_access_key: &str,
    bucket_name: &str,
    object_key: &str,
    region: &str,
    path_to_download: &str,
) {
    match Path::new(path_to_download.clone()).exists() {
        true => info!(
            "File {} already exist, nothing to do",
            path_to_download.clone()
        ),
        false => {}
    }
}