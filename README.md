# Curtail: pipe stdin to a fixed-size log file

Have you ever met a situation that a program output's lots of logs that could
be dropped, but redirect logs to `/dev/null` is unacceptable cause most recent
logs are invaluable if the program quit for unknown reasons.

Meet `curtail`, which could pipe the logs into a fixed size file.

It lets you keep the most recent logs with fixed disk spaces.

## Usage

```sh
<some program> | curtail [-s size] <output file>
```

You could specify the size of output file by:

```
-size  size limit of the log file, will be upcast to be multiples of block size, default to 16K
```

## How it works?

`curtail` would remove the first blocks(typically `4K`) of the file if the next
write to the file would make the file size exceeds the specified limit.

There is no standard efficient way to delete content in front of the file.
Under Linux however, we could call `fallocate` with `FALLOC_FL_COLLAPSE_RANGE`
option to achieve this. Unfortunately this options is filesystem specific,
under Linux 3.15, it only support EXT4 and XFS.

## References

- https://github.com/Comcast/Infinite-File-Curtailer The origin of the idea.
    It's a C version. I made this rust version mainly because I need an ARM
    version for my router.
- https://man7.org/linux/man-pages/man2/fallocate.2.html man page for
    `fallocate`
