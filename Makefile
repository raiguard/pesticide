PREFIX = $(DESTDIR)/usr/local
BINDIR = $(PREFIX)/bin
MANDIR = $(PREFIX)/share/man

FILES = $(shell find . -type f -name "*.go")

all: pesticide

pesticide: $(FILES)
	go build

install:
	install -d $(BINDIR)
	install -pm 0755 pesticide $(BINDIR)/

uninstall:
	rm -f $(BINDIR)/pesticide

clean:
	go clean

.PHONY: install uninstall clean
