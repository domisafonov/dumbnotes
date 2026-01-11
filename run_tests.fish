#!/usr/bin/env fish

argparse 'h/help' 'C/nocapture' -- $argv

if set -ql _flag_h
    set -l self $argv[0]
    echo "Usage: $self [-C] [--nocapture]" >&2
end

set -l extra_bin_args
if set -ql _flag_C
    set extra_bin_args $extra_bin_args '--nocapture'
end

set -l script_dir (path resolve (status --current-filename)/..)
$script_dir/cargow.fish test -- --test-threads=1 $extra_bin_args
