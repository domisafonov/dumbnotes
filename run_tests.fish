#!/usr/bin/env fish

switch (uname)
case OpenBSD
    LLVM_CONFIG_PATH=$(which llvm-config-21) LD_LIBRARY_PATH=$($LLVM_CONFIG_PATH --libdir) cargo test -- --test-threads=1
case '*'
    cargo test -- --test-threads=1
end
