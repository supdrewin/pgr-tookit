use pgr_dl::prelude::*;

use std::{env, sync::Arc};

use console::Term;
use indicatif::MultiProgress;
use tokio::runtime::Builder;

fn main() -> DynResult<()> {
    let cli = Cli::new();

    let mut rt = Builder::new_multi_thread();

    let rt = match cli.threads {
        Some(threads) => rt.worker_threads(threads),
        None => &mut rt,
    }
    .enable_all()
    .build()?;

    rt.block_on(async {
        let mp = MultiProgress::new();

        let dest_dir = Arc::new(
            cli.path
                .unwrap_or(env::current_dir()?)
                .join("Punishing Gray Raven Game"),
        );

        let index_json = pgr_dl::get_response!(index.json, INDEX_JSON_URL[0]);

        let resources = &index_json.default.resources;
        let base_path = &index_json.default.resources_base_path;

        let host = &index_json
            .default
            .cdn_list
            .get(cli.mirror.unwrap_or_default())
            .unwrap_or(&index_json.default.cdn_list[0])
            .url;

        let resource_json = pgr_dl::get_response!(resource.json, format!("{host}{resources}"));

        let mut pool = Pool::new()?;
        let mut tasks = vec![];

        for resource in resource_json.resource {
            let dest_dir = dest_dir.clone();
            let base_url = format!("{host}{base_path}");

            let sender = pool.sender.clone();
            let mp = mp.clone();

            pgr_dl::while_err! { pool.watcher.changed().await }
            pgr_dl::while_err! { sender.send(PoolOp::Attach).await }

            tasks.push(rt.spawn(async move {
                let helper = ResourceHelper::new(resource, &base_url, dest_dir.to_str().unwrap())
                    .with_progress_bar()
                    .with_multi_progress(mp);

                pgr_dl::while_err! { helper.download().await }
                pgr_dl::while_err! { sender.send(PoolOp::Dettach).await }
            }));
        }

        pgr_dl::wait_all!(tasks, 1);

        println!("All the resources are downloaded!");
        println!("Press any key to continue...");

        Ok(Term::stdout().read_key().map(|_| ())?)
    })
}
