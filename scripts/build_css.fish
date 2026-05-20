#!/usr/bin/env fish

function make_css -a src dst extra_args
    pnx @tailwindcss/cli --input=$src --output=$dst $extra_args
    or exit 1
end

set -l css_dirs dumbnotesd-web-css
set -l project_dir (path resolve (status dirname)/..)
set -l css_dirs $project_dir/$css_dirs

for css_dir in $css_dirs
    pushd $css_dir
    for css in *.css
        set -l variant (path extension (path change-extension '' $css))
        if test $variant = .debug -o $variant = .release
            continue
        end

        make_css $css (path change-extension debug.css $css)
        make_css $css (path change-extension release.css $css) --minify
    end
    popd
end
