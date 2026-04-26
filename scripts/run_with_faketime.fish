#!/usr/bin/env fish

argparse --strict-longopts\
    --exclusive t,C --exclusive e,C\
    h/help\
    t/timestamp-file=\
    e/executable=\
    C/cleanup\
    -- $argv
or return 1

if set -ql _flag_h
    set -l self (status basename)
    echo "Usage: $self [-h] -t path/to/timestamp/file -e path/to/executable -- [arguments..]" >&2
    echo "Usage: $self [-h] -C" >&2
    return
end


if set -ql _flag_C
    rm -vf (path resolve /dev/shm/{faketime_shm_,sem.faketime_sem_,faketime_lock_}*)
    return
end


if not set -ql _flag_t; or not set -ql _flag_e
    echo "'-t' and '-e' flags are required" >&2
    return 1
end
set -l timestamp_file $_flag_t
set -l executable $_flag_e


set -l repo_base (path resolve (status dirname)/..)
set -l faketime_src $repo_base/libfaketime
set -l faketime_so
if test -d $faketime_src
    # build libfaketime from source if the submodule is initialized
    set -l make_libfaketime make --silent --directory=$faketime_src
    if not $make_libfaketime
        echo failed to build libfaketime: >&2
        $make_libfaketime
        return 1
    end
    set faketime_so $faketime_src/src/libfaketime.so.1
else
    # fallback to system-installed libfaketime
    set faketime_so (ldconfig -p | rg libfaketime.so.1 | cut -s -d ' ' -f 4)
    if test ! -f $faketime_so
        echo could not find libfaketime using ldconfig, aborting >&2
        return 1
    end
end

if test ! -f $timestamp_file; and not touch -m $timestamp_file
    echo "unable to find or create the timestamp file at '$timestamp_file', aborting" >&2
    return 1
end

LD_PRELOAD=$faketime_so \
FAKETIME=% \
FAKETIME_FOLLOW_FILE=$timestamp_file \
FAKETIME_NO_CACHE=1 \
exec $executable $argv
