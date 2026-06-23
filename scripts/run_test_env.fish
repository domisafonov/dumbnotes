#!/usr/bin/env fish

argparse --strict-longopts --exclusive u,s,c --exclusive p,s,c\
    h/help\
    u/username\
    p/password\
    s/stop\
    c/cleanup\
    -- $argv
or return 1

if set -ql _flag_h
    set -l self (status basename)
    echo "Usage: $self [-hups] [--help] [--username] [--password] [--stop]" >&2
    return
end

if set -ql _flag_s
    kill -INT (pgrep dumbnotesd 2>/dev/null) &>/dev/null
    return
end

if set -ql _flag_c
    set -l paths (path resolve /tmp/dumbnotes.dev*)
    if test -z "$paths"
        return
    end
    find $paths -type d -exec chmod u+wx \{\} \; &>/dev/null
    rm -rfv $paths
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

echo -n 'Compiling ... ' >&2
cargo build\
    --profile test-env\
    --config 'build.rustflags=["--cfg=test_env", "-Awarnings"]'\
    --bin dumbnotesd\
    --bin dumbnotesd-api\
    --bin dumbnotesd-web\
    --bin dumbnotesd-auth\
    --bin dumbnotesd-storage\
    --bin dumbnotes-gen\
    --quiet
if test $status -eq 0
    echo 'finished' >&2
else
    echo 'failed' >&2
    return 1
end

set -l mktemp_location_arg
switch (uname -s)
case OpenBSD
    set mktemp_location_arg -t
case '*'
    set mktemp_location_arg --tmpdir
end

umask 077

set -g basedir (mktemp -d $mktemp_location_arg dumbnotes.dev.XXXXXX)
    or return 1
echo "Created $basedir"
function on_exit --on-event fish_exit
    find $basedir -type d -exec chmod u+wx \{\} \;
    rm -rf $basedir
end

set -l secrets_dir $basedir/private
mkdir $secrets_dir
    or return 1
set -l data_dir $basedir/data
mkdir $data_dir
    or return 1
set -l private_data_dir $data_dir/private
mkdir $private_data_dir
    or return 1
set -l notes_dir $data_dir/notes
mkdir $notes_dir
    or return 1

chmod 500 $data_dir

set -l config_file $basedir/dubmnotes.toml
set -l user_db $secrets_dir/users.toml
set -l session_db $private_data_dir/session.toml

echo -n "jwt_private_key = \"$secrets_dir/jwt_private_key.json\"
jwt_public_key = \"$basedir/jwt_public_key.json\"
pepper_path = \"$secrets_dir/pepper.b64\"
user_db = \"$user_db\"
data_directory = \"$data_dir\"
" > $config_file

set -l project_dir (path resolve (status dirname)/..)
set -l bin_dir $project_dir/target/test-env
$bin_dir/dumbnotes-gen --config-file=$config_file --generate-jwt-key
    or return 1
$bin_dir/dumbnotes-gen --config-file=$config_file --generate-pepper
    or return 1

chmod 500 $basedir

set -l password_hash (echo -n $password | $bin_dir/dumbnotes-gen --config-file=$config_file --no-repeat)
    or return 1
echo -n "[[user]]
username = \"$username\"
hash = \"$password_hash\"
" > $user_db
chmod 400 $user_db

chmod 500 $secrets_dir

touch $session_db
chmod 600 $session_db

tree -a $basedir
    or return 1

$bin_dir/dumbnotesd --config-file=$config_file --no-fork
