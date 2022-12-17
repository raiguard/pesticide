PREFIX = $(DESTDIR)/usr/local
BINDIR = $(PREFIX)/bin
MANDIR = $(PREFIX)/share/man

all: pest docs

pest: *.go
	go build -o pest

docs: pest.1

pest.1: pest.1.scd
	scdoc < $< > $@

install:
	install -d \
		$(BINDIR) \
		$(MANDIR)/man1/
	install -pm 0755 pest $(BINDIR)/
	install -pm 0644 pest.1 $(MANDIR)/man1/

uninstall:
	rm -f \
		$(BINDIR)/pest \
		$(MANDIR)/man1/pest.1

clean:
	go clean
	rm -f pest
	rm -f pest.1

.PHONY: docs install uninstall clean
