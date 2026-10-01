rootdir := ''
prefix := '/usr'
usrdir := absolute_path(clean(rootdir / prefix))

# Keep the checked and installed protocol set explicit.
unstable-protocols := '''
    unstable/cosmic-a11y-unstable-v1.xml
    unstable/cosmic-corner-radius-unstable-v1.xml
    unstable/cosmic-image-capture-source-unstable-v1.xml
    unstable/cosmic-output-management-unstable-v1.xml
    unstable/cosmic-overlap-notify-unstable-v1.xml
    unstable/cosmic-toplevel-info-unstable-v1.xml
    unstable/cosmic-toplevel-management-unstable-v1.xml
    unstable/cosmic-workspace-unstable-v1.xml
    unstable/cosmic-workspace-unstable-v2.xml
'''
protocol-args := replace(unstable-protocols, "\n", ' ')

default: check

# Validate the protocols with wayland-scanner.
check:
    ./check.sh {{ protocol-args }}

# Remove the generated pkg-config file.
clean:
    rm -f cosmic-protocols.pc

# Generate pkg-config metadata using the final prefix, not the staging root.
generate-pc:
    sed \
        -e {{ quote('s:@prefix@:' + prefix + ':g') }} \
        -e 's:@datadir@:${datarootdir}:g' \
        -e 's:@datarootdir@:${prefix}/share:g' \
        < cosmic-protocols.pc.in > cosmic-protocols.pc

# Install the explicitly selected unstable protocols.
install-unstable:
    for protocol in {{ protocol-args }}; do \
        install -Dm0644 "$protocol" {{ quote(usrdir / 'share/cosmic-protocols') }}/"$protocol"; \
    done

# Install pkg-config metadata.
install-pc: generate-pc
    install -Dm0644 cosmic-protocols.pc {{ quote(usrdir / 'share/pkgconfig/cosmic-protocols.pc') }}

# Install protocols and pkg-config metadata.
install: install-unstable install-pc
