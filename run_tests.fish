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
$script_dir/cargow.fish test --no-fail-fast --lib --bins --benches --examples -- $extra_bin_args
$script_dir/cargow.fish test --profile integration-test --no-fail-fast --test '*' -- --test-threads=1 $extra_bin_args
$script_dir/cargow.fish test --doc -- $extra_bin_args
