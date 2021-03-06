use crate::{
    channel::{Channel, ChannelInput},
    error::ChatResult,
    message::handlers::add_chunk_path,
    utils::to_timestamp,
};
use hdk3::prelude::*;
use link::Link;

use super::{ChannelData, ChannelInfo, ChannelInfoTag, ChannelList, ChannelListInput};

/// Create a new channel
/// This effectively just stores channel info on the
/// path that is `category:channel_id`
pub(crate) fn create_channel(channel_input: ChannelInput) -> ChatResult<ChannelData> {
    let ChannelInput { name, channel } = channel_input;

    // Create the path for this channel
    let path: Path = channel.clone().into();
    path.ensure()?;

    // Create the channel info
    let info = ChannelInfo {
        // This agent
        created_by: agent_info()?.agent_initial_pubkey,
        // Right now
        created_at: to_timestamp(sys_time()?),
        name,
    };

    // Commit the channel info
    create_entry(&info)?;
    let info_hash = hash_entry(&info)?;

    // link the channel info to the path
    create_link(path.hash()?, info_hash, ChannelInfoTag::tag())?;

    // Return the channel and the info for the UI
    Ok(ChannelData::new(channel, info, 0))
}

pub(crate) fn list_channels(list_channels_input: ChannelListInput) -> ChatResult<ChannelList> {
    // Get the category path
    let path = Path::from(list_channels_input.category);

    // Get any channels on this path
    let links = path.children()?.into_inner();
    let mut channels = Vec::with_capacity(links.len());

    // For each channel get the channel info links and choose the latest
    for tag in links.into_iter().map(|link| link.tag) {
        // Path links have their full path as the tag so
        // we don't need to get_links on the child.
        // The tag can be turned into the channel path
        let channel_path = Path::try_from(&tag)?;

        // Turn the channel path into the channel
        let channel = Channel::try_from(&channel_path)?;

        // Get any channel info links on this channel
        let channel_info =
            get_links(channel_path.hash()?, Some(ChannelInfoTag::tag()))?.into_inner();

        // Find the latest
        let latest_info = channel_info
            .into_iter()
            .fold(None, |latest: Option<Link>, link| match latest {
                Some(latest) => {
                    if link.timestamp > latest.timestamp {
                        Some(link)
                    } else {
                        Some(latest)
                    }
                }
                None => Some(link),
            });

        // If there is none we will skip this channel
        let latest_info = match latest_info {
            Some(l) => l,
            None => continue,
        };

        // find the latest chunk
        let path: Path = channel.clone().into();
        let links = path.children()?.into_inner();
        let mut chunk: u32 = 0;
        for link in links.into_iter() {
            let chunk_path = Path::try_from(&link.tag)?;
            let chunks: Vec<_> = chunk_path.into();
            if let Some(c) = chunks
                .last()
                .and_then(|c| String::try_from(c).ok()?.parse::<u32>().ok())
            {
                if c > chunk {
                    chunk = c;
                }
            }
        }

        // Get the actual channel info entry
        if let Some(element) = get(latest_info.target, GetOptions::content())? {
            if let Some(info) = element.into_inner().1.to_app_option()? {
                // Construct the channel data from the channel and info
                channels.push(ChannelData {
                    channel,
                    info,
                    latest_chunk: chunk,
                });
            }
        }
    }
    // Return all the channels data to the UI
    Ok(channels.into())
}

// Note: This function can get very heavy
pub(crate) fn channel_stats(list_channels_input: ChannelListInput) -> ChatResult<(usize, usize)> {
    let channel_path = Path::from(list_channels_input.category);
    let channel_links = channel_path.children()?.into_inner();
    let mut msg_links: Vec<Link> = Vec::new();
    for tag in channel_links.clone().into_iter().map(|link| link.tag) {
        let channel_path = Path::try_from(&tag)?;
        let channel = Channel::try_from(&channel_path)?;
        let mut chunk = 0;
        loop {
            let message_path: Path = channel.clone().into();
            // Add the chunk component
            let path = add_chunk_path(message_path, chunk)?;

            // Get the actual hash we are going to pull the messages from
            let channel_entry_hash = path.hash()?;

            let mut links = get_links(channel_entry_hash.clone(), None)?.into_inner();
            if links.clone().len() == 0 {
                break;
            }
            msg_links.append(&mut links);
            chunk += 1
        }
    }
    Ok((channel_links.len(), msg_links.len()))
}
