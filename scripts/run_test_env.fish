#!/usr/bin/env fish

argparse --strict-longopts\
    h/help\
    u/username\
    p/password\
    -- $argv
or return 1

if set -ql _flag_h
    set -l self (status basename)
    echo "Usage: $self [-hup] [--help] [--username] [--password]" >&2
    return
end

set -l username
if not set -ql _flag_u
    echo 'Using default username "abc"' >&2
    set username abc
end

set -l password
if not set -ql _flag_p
    echo 'Using default password "123"' >&2
    set password 123
end

cargo build\
    --bin dumbnotesd\
    --bin dumbnotesd-auth\
    --bin dumbnotes-gen

mktemp
