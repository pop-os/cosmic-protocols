PREFIX=/usr
DATADIR=$${datarootdir}
DATAROOTDIR=$${prefix}/share

unstable_protocols = \
	unstable/cosmic-screencopy-unstable-v1.xml \
	unstable/cosmic-export-dmabuf-unstable-v1.xml \
	unstable/cosmic-toplevel-info-unstable-v1.xml \
	unstable/cosmic-toplevel-management-unstable-v1.xml \
	unstable/cosmic-workspace-unstable-v1.xml \

check: $(unstable_protocols)
	./check.sh $(unstable_protocols)

clean:
	rm -f cosmic-protocols.pc

cosmic-protocols.pc: cosmic-protocols.pc.in
	sed \
		-e 's:@prefix@:$(PREFIX):g' \
		-e 's:@datadir@:$(DATADIR):g' \
		-e 's:@datarootdir@:$(DATAROOTDIR):g' \
		<$< >$@

install-unstable: $(unstable_protocols)
	mkdir -p $(DESTDIR)$(PREFIX)/share/cosmic-protocols/unstable
	for protocol in $^ ; \
	do \
		install -Dm644 $$protocol \
			$(DESTDIR)$(PREFIX)/share/cosmic-protocols/$$protocol ; \
	done

install-pc: cosmic-protocols.pc
	mkdir -p $(DESTDIR)$(PREFIX)/share/pkgconfig/
	install -Dm644 cosmic-protocols.pc \
		$(DESTDIR)$(PREFIX)/share/pkgconfig/cosmic-protocols.pc

install: install-unstable install-pc
