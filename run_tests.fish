#!/usr/bin/env fish

argparse -x u,i,d h/help C/nocapture u/units i/integration d/doc -- $argv
or return

if set -ql _flag_h
    set -l self (status basename)
    echo "Usage: $self [-hCuid] [--help] [--nocapture] [--units] [--integration] [--doc]" >&2
    return
end

set -l extra_bin_args
if set -ql _flag_C
    set extra_bin_args $extra_bin_args '--nocapture'
end

set -l all
if set -ql _flag_u; or set -ql _flag_i; or set -ql _flag_d
    set -e all
end


set -l success
if set -ql all; or set -ql _flag_u
    cargo test --no-fail-fast --lib --bins --benches\
        --examples -- $extra_bin_args
    or set -e success
end
if set -ql success; and begin set -ql all; or set -ql _flag_i; end
    cargo test --profile integration-test\
        --config 'build.rustflags=["--cfg=integration_test"]' --no-fail-fast\
        --test '*' -- --test-threads=1 $extra_bin_args
    or set -e success
end
if set -ql success; and begin set -ql all; or set -ql _flag_d; end
    cargo test --doc -- $extra_bin_args
end
