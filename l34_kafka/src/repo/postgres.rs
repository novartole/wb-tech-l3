use super::{ListenEvent, Store};
use crate::{
    error::Error,
    model::{Id, Product, User},
};
use bb8::Pool;
use bb8_postgres::PostgresConnectionManager;
use futures_channel::mpsc;
use futures_util::{stream, FutureExt, StreamExt};
use serde::de::DeserializeOwned;
use std::str::FromStr;
use tokio_postgres::{AsyncMessage, Config, NoTls};
use tracing::{debug, error, trace};

#[derive(Clone)]
pub struct PgRepo {
    pool: Pool<PostgresConnectionManager<NoTls>>,
}

impl PgRepo {
    pub async fn try_new(params: &str) -> Result<Self, tokio_postgres::Error> {
        let config = Config::from_str(params)?;
        let manager = PostgresConnectionManager::new(config, NoTls);
        let pool = Pool::builder().build(manager).await?;

        Ok(Self { pool })
    }
}

impl Store for PgRepo {
    // users

    async fn create_user(&self, user: User) -> Result<(), Error> {
        self.pool
            .get()
            .await?
            .execute(
                "INSERT INTO users (name, email) VALUES ($1, $2)",
                &[&user.name, &user.email],
            )
            .await?;

        Ok(())
    }

    async fn update_user(&self, user_id: &Id, user: User) -> Result<(), Error> {
        if 0 == self
            .pool
            .get()
            .await?
            .execute(
                "UPDATE users SET name = $1, email = $2 WHERE id = $3",
                &[&user.name, &user.email, user_id],
            )
            .await?
        {
            return Err(Error::NotFound("User"));
        }

        Ok(())
    }

    async fn delete_user(&self, user_id: &Id) -> Result<(), Error> {
        if 0 == self
            .pool
            .get()
            .await?
            .execute("DELETE FROM users WHERE id = $1", &[user_id])
            .await?
        {
            return Err(Error::NotFound("User"));
        }

        Ok(())
    }

    // products

    async fn create_product(&self, product: Product) -> Result<(), Error> {
        self.pool
            .get()
            .await?
            .execute(
                "INSERT INTO products (name, price) VALUES ($1, $2)",
                &[&product.name, &product.price],
            )
            .await?;

        Ok(())
    }

    async fn update_product(&self, product_id: &Id, product: Product) -> Result<(), Error> {
        if 0 == self
            .pool
            .get()
            .await?
            .execute(
                "UPDATE products SET name = $1, price = $2 WHERE id = $3",
                &[&product.name, &product.price, product_id],
            )
            .await?
        {
            return Err(Error::NotFound("Product"));
        }

        Ok(())
    }

    async fn delete_product(&self, product_id: &Id) -> Result<(), Error> {
        if 0 == self
            .pool
            .get()
            .await?
            .execute("DELETE FROM products WHERE id = $1", &[product_id])
            .await?
        {
            return Err(Error::NotFound("Product"));
        }

        Ok(())
    }
}

pub struct PgEventListener<'a> {
    db_params: &'a str,
}

impl<'a> PgEventListener<'a> {
    pub fn new(db_params: &'a str) -> Self {
        Self { db_params }
    }
}

impl<'a> ListenEvent for PgEventListener<'a> {
    async fn listen<T>(
        &self,
        event: impl AsRef<str>,
        handle: impl Fn(T) + Send + 'static,
    ) -> Result<&Self, Error>
    where
        T: DeserializeOwned,
    {
        let (msg_tx, mut msg_rx) = mpsc::unbounded();
        let (client, mut conn) = tokio_postgres::connect(self.db_params, NoTls).await?;
        debug!(db_params = self.db_params, "connected to database");

        tokio::spawn(
            stream::poll_fn(move |cx| {
                conn.poll_message(cx).map_err(|error| {
                    error!("{:?}", error);
                    panic!("Failed getting messages from Postgres");
                })
            })
            .forward(msg_tx)
            .map(|sent| {
                sent.inspect_err(|why| error!("failed sending message into stream: {:?}", why))
                    .expect("Processing message from Postgres")
            }),
        );
        trace!("spawned notification sender");

        tokio::spawn({
            let listening = format!("LISTEN {};", event.as_ref());

            async move {
                if let Err(why) = client.batch_execute(&listening).await {
                    error!("failed to start getting notifications: {:?}", why);
                    return;
                }

                while let Some(msg) = msg_rx.next().await {
                    if let AsyncMessage::Notification(n) = msg {
                        trace!("got notification from Postgres");
                        match serde_json::from_str::<T>(n.payload()) {
                            Err(why) => {
                                error!("failed to start getting notifications: {:?}", why);
                                return;
                            }
                            Ok(t) => handle(t),
                        }
                    } else {
                        debug!(message = ?msg, "skip");
                    }
                }
            }
        });
        trace!("spawned notification receiver");

        Ok(self)
    }
}
