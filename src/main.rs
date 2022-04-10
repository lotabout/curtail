use getopts::Options;
use nix::fcntl::{fallocate, FallocateFlags};
use nix::sys::stat::stat;
use std::env;
use std::error::Error;
use std::io::prelude::*;
use std::io::{Seek, SeekFrom};
#[cfg(unix)]
use std::os::unix::io::AsRawFd;
use std::path::Path;
use nix::libc::off_t;

fn print_usage(program: &str, opts: Options) {
    let brief = format!("Usage: {} [options] LOG_FILE", program);
    print!("{}", opts.usage(&brief));
}

fn parse_size(size_spec: &str) -> Result<usize, Box<dyn Error>> {
    if size_spec.is_empty() {
        return Err("`size` should not be empty".into());
    }

    let unit_pos = size_spec
        .find(char::is_alphabetic)
        .unwrap_or(size_spec.len());
    let size: usize = size_spec[0..unit_pos].parse()?;
    let unit_spec = &size_spec[unit_pos..];
    let unit = match unit_spec {
        "" => 1,
        "k" | "K" => 1024,
        "m" | "M" => 1024 * 1024,
        "g" | "G" => 1024 * 1024 * 1024,
        _ => return Err(format!("could not parse size spec: {}", unit_spec).into()),
    };

    Ok(size * unit)
}

fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<String> = env::args().collect();
    let program = args[0].clone();

    let mut opts = Options::new();
    opts.optopt(
        "s",
        "size",
        "size limit of the log file, will be upcast to be multiples of block size",
        "512, 2K, 10M, 1G",
    );
    opts.optflag("h", "help", "print help menu");
    let matches = opts.parse(&args[1..])?;

    // print help menu
    if matches.opt_present("h") {
        print_usage(&program, opts);
        return Ok(());
    }

    // missing filename
    let filename = if !matches.free.is_empty() {
        matches.free[0].clone()
    } else {
        println!("ERROR: missing LOG_FILE options");
        print_usage(&program, opts);
        return Ok(());
    };

    let size_spec = matches.opt_str("s").unwrap_or_else(|| "16K".to_string());
    let filesize_limit = parse_size(&size_spec)?;

    process(&filename, filesize_limit)
}

fn process(filename: &str, size_limit: usize) -> Result<(), Box<dyn Error>> {
    let mut fp = std::fs::File::options()
        .write(true)
        .create(true)
        .open(filename)?;

    // justify block size (should be at least twice the block size)
    let blksize = stat(Path::new(filename))?.st_blksize as usize;
    let limit = 2.max(size_limit / blksize) * blksize;

    let stdin = std::io::stdin();
    let mut stdin_handle = stdin.lock();
    let mut buffer = [0; 1024];

    loop {
        let size_read = stdin_handle.read(&mut buffer)?;
        if size_read == 0 {
            break; // EOF
        }
        write_with_curtail(&mut fp, &buffer[0..size_read], limit, blksize)?;
    }

    Ok(())
}

fn write_with_curtail(
    fp: &mut std::fs::File,
    buf: &[u8],
    size_limit: usize,
    blksize: usize,
) -> Result<(), Box<dyn Error>> {
    let pos = fp.stream_position()? as usize;

    // collapse the first N blocks to maintain total size limit
    if pos + buf.len() > size_limit {
        let num_blocks = (pos + buf.len() - size_limit + (blksize - 1)) / blksize;
        let target_offset = (num_blocks * blksize) as off_t;
        fallocate(
            fp.as_raw_fd(),
            FallocateFlags::FALLOC_FL_COLLAPSE_RANGE,
            0,
            target_offset,
        )?;
        let _new_pos = fp.seek(SeekFrom::End(0))?;
    }

    fp.write_all(buf)?;
    Ok(())
}
