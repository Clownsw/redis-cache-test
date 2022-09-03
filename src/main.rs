use chrono::Local;
use fast_log::{Config, FastLogFormat};
use fast_log::filter::NoFilter;
use fast_log::plugin::console::ConsoleAppender;
use log::LevelFilter;
use rbatis::{crud, Rbatis};
use rbdc_mysql::driver::MysqlDriver;
use redis::AsyncCommands;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Blog {
    pub id: i64,
    pub user_id: i64,
    pub sort_id: i32,
    pub title: String,
    pub description: String,
    pub content: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BlogInfo {
    pub id: i64,
    pub sort_id: i32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BlogContent {
    pub user_id: i64,
    pub title: String,
    pub description: String,
    pub content: String,
}

impl Into<(BlogInfo, BlogContent)> for Blog {
    fn into(self) -> (BlogInfo, BlogContent) {
        (BlogInfo {
            id: self.id,
            sort_id: self.sort_id,
        },
         BlogContent {
             user_id: self.user_id,
             title: self.title,
             description: self.description,
             content: self.content,
         })
    }
}

impl From<(BlogInfo, BlogContent)> for Blog {
    fn from(v: (BlogInfo, BlogContent)) -> Self {
        Blog {
            id: v.0.id,
            user_id: v.1.user_id,
            sort_id: v.0.sort_id,
            title: v.1.title,
            description: v.1.description,
            content: v.1.content,
        }
    }
}

crud!(Blog {}, "m_blog");

pub async fn build_data(rb: &mut Rbatis, client: &mut redis::aio::Connection) -> Result<(), Box<dyn std::error::Error>> {
    let blogs = Blog::select_all(rb)
        .await?;

    for blog in blogs {
        // let json_str = serde_json::to_string(&blog)?;
        // client.set::<_, _, ()>(blog.id, json_str).await?;
        // client.del::<_, ()>(blog.id).await?;

        let (blog_info, blog_content) = blog.into();
        client.lpush::<_, _, ()>(format!("blog:list:sort:info:{}", blog_info.sort_id), serde_json::to_string(&blog_info)?).await?;
        client.lpush::<_, _, ()>(format!("blog:list:sort:content:{}", blog_info.sort_id), serde_json::to_string(&blog_content)?).await?;
        // client.del::<_, ()>("").await?;
    }

    Ok(())
}

pub async fn get_blog_vec_by_sort_id(sort_id_vec: Vec<i32>, client: &mut redis::aio::Connection) -> Result<(), Box<dyn std::error::Error>> {
    let mut blogs: Vec<Blog> = Vec::new();

    let start_time = Local::now().timestamp();
    for sort_id in sort_id_vec.iter() {
        let key = format!("blog:list:sort:info:{}", sort_id);
        let key2 = format!("blog:list:sort:content:{}", sort_id);
        let len = client.llen::<_, isize>(&key).await?;

        if len > 0 {
            for i in 0..len {
                let blog = Blog::from((
                    serde_json::from_str::<BlogInfo>(client.lindex::<_, String>(&key, i).await?.as_str())?,
                    serde_json::from_str::<BlogContent>(client.lindex::<_, String>(&key2, i).await?.as_str())?
                ));
                blogs.push(blog);
                // println!("{:#?}", blog);
            }
        }
    }

    let end_time = Local::now().timestamp();

    println!("{} ms", end_time - start_time);
    println!("{}, {}", start_time, end_time);
    println!("{:#?}", blogs);

    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    fast_log::init(Config {
        appends: vec![Box::new(ConsoleAppender {})],
        level: LevelFilter::Debug,
        filter: Box::new(NoFilter {}),
        format: Box::new(FastLogFormat::new()),
        chan_len: None,
    })?;

    let mut rb = rbatis::Rbatis::new();
    rb.init(MysqlDriver {}, "mysql://root:123123@localhost:3333/vueblog")?;

    let client = redis::Client::open("redis://127.0.0.1")?;

    let count: i64 = rb.fetch_decode("SELECT COUNT(1) FROM m_blog", vec![])
        .await
        .unwrap();

    println!("count {}", count);

    let mut client = client.get_async_connection().await?;
    // build_data(&mut rb, &mut client).await?;
    get_blog_vec_by_sort_id(vec![1, 3, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17], &mut client).await?;


    Ok(())
}
