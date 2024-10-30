use anyhow::{anyhow, Context as _, Result};
use futures::Stream;
use inotify::{EventMask, Inotify, WatchMask};
use std::{
    ffi::{OsStr, OsString},
    future::Future,
    path::{Path, PathBuf},
    pin::Pin,
    task::{Context, Poll},
};
use tokio::{
    pin,
    sync::mpsc::{self, UnboundedReceiver},
};
use tracing::error;

pub struct Notifier<'a> {
    dir: &'a Path,
    rx: UnboundedReceiver<Option<Result<OsString>>>,
}

pub async fn try_watch(dir: &Path) -> Result<Notifier<'_>> {
    let mut inotify = {
        let inotify = Inotify::init().context("Creating Inotify instance")?;
        inotify
            .watches()
            .add(dir, WatchMask::MODIFY)
            .context("Adding watcher")?;
        inotify
    };

    let (tx, rx) = mpsc::unbounded_channel();

    tokio::task::spawn_blocking(move || {
        let mut buffer = [0u8; 4096];

        loop {
            let events = match inotify
                .read_events_blocking(&mut buffer)
                .inspect_err(|why| error!("failed reading events: {}", why))
            {
                Ok(events) => events,
                _ => {
                    break tx
                        .send(None)
                        .inspect_err(|why| error!("failed sending to worker: {:?}", why))
                }
            };

            for event in events {
                let is_modified_file = event.mask.contains(EventMask::MODIFY)
                    && !event.mask.contains(EventMask::ISDIR);

                if is_modified_file {
                    let res = event
                        .name
                        .map(OsStr::to_os_string)
                        .ok_or(anyhow!("failed to get file name"));

                    if tx
                        .send(Some(res))
                        .inspect_err(|why| error!("failed sending to worker: {:?}", why))
                        .is_err()
                    {
                        break;
                    }
                }
            }
        }
    });

    Ok(Notifier { dir, rx })
}

impl<'a> Stream for Notifier<'a> {
    type Item = Result<PathBuf>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let dir = *unsafe { self.as_mut().map_unchecked_mut(|this| &mut this.dir) };

        let recv = self.get_mut().rx.recv();
        pin!(recv);

        match recv.poll(cx) {
            Poll::Ready(Some(Some(try_str))) => {
                let try_path = try_str.map(|str| Path::new(dir).join(str));
                let res = Some(try_path);
                Poll::Ready(res)
            }
            Poll::Pending => Poll::Pending,
            _ => Poll::Ready(None),
        }
    }
}
