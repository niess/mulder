# Compiler flags
CC=     gcc
CFLAGS= -O3 -g -Wall -std=c99

# OS dependent flags
UNAME_S := $(shell uname -s)
ifeq ($(UNAME_S),Darwin)
    SOEXT=  dylib
    LIB=    libmulder.$(SOEXT)
    LD=     $(CC) -dynamiclib -Wl,-install_name,@rpath/$(LIB)
    RPATH=  -Wl,-rpath,@loader_path/../lib
else
    SOEXT=  so
    LIB=    libmulder.$(SOEXT)
    LD=     $(CC) -shared
    RPATH=  '-Wl,-rpath,$$ORIGIN/../lib'
endif

# Version flags
VERSION_MAJOR= $(shell grep MULDER_VERSION_MAJOR src/mulder.h | cut -d' ' -f3)
VERSION_MINOR= $(shell grep MULDER_VERSION_MINOR src/mulder.h | cut -d' ' -f3)
VERSION_PATCH= $(shell grep MULDER_VERSION_PATCH src/mulder.h | cut -d' ' -f3)

LIB_SHORTNAME= $(LIB).$(VERSION_MAJOR)
LIB_FULLNAME=  $(LIB_SHORTNAME).$(VERSION_MINOR).$(VERSION_PATCH)


# C library compilation
PUMAS_DIR= deps/pumas
TURTLE_DIR= deps/turtle

LIB_CFLAGS= $(CFLAGS) -shared -fPIC \
            -I$(PUMAS_DIR)/include -I$(TURTLE_DIR)/include

LIB_DEPS = src/pumas.o \
           src/turtle_client.o \
           src/turtle_ecef.o \
           src/turtle_error.o \
           src/turtle_io.o \
           src/turtle_io_geotiff16.o \
           src/turtle_io_grd.o \
           src/turtle_io_png16.o \
           src/turtle_list.o \
           src/turtle_map.o \
           src/turtle_projection.o \
           src/turtle_stack.o \
           src/turtle_stepper.o \
           src/turtle_jsmn.o \
           src/turtle_tinydir.o

.PHONY: lib
lib: lib/$(LIB_FULLNAME) \
     lib/$(LIB_SHORTNAME) \
     lib/$(LIB)

lib/$(LIB_FULLNAME): src/mulder.c src/mulder.h $(LIB_DEPS) | libdir
	$(LD) -o $@ $(LIB_CFLAGS) $^ -ldl -lm

lib/$(LIB_SHORTNAME): lib/$(LIB_FULLNAME)
	@ln -fs $(LIB_FULLNAME) $@

lib/$(LIB): lib/$(LIB_SHORTNAME)
	@ln -fs $(LIB_SHORTNAME) $@

.PHONY: libdir
libdir:
	@mkdir -p lib

src/pumas.o: $(PUMAS_DIR)/src/pumas.c $(PUMAS_DIR)/include/pumas.h
	$(CC) $(LIB_CFLAGS) -o $@ -c $<

TURTLE_CFLAGS= $(LIB_CFLAGS) \
               -DTURTLE_NO_ASC -DTURTLE_NO_HGT \
               -I$(TURTLE_DIR)/src

src/turtle_%.o: $(TURTLE_DIR)/src/turtle/%.c $(TURTLE_DIR)/src/turtle/%.h
	$(CC) $(TURTLE_CFLAGS) -o $@ -c $<

src/turtle_%.o: $(TURTLE_DIR)/src/turtle/%.c
	$(CC) $(TURTLE_CFLAGS) -o $@ -c $<

src/turtle_io_%.o: $(TURTLE_DIR)/src/turtle/io/%.c
	$(CC) $(TURTLE_CFLAGS) -o $@ -c $<

src/turtle_%.o: $(TURTLE_DIR)/src/deps/%.c $(TURTLE_DIR)/src/deps/%.h
	$(CC) $(TURTLE_CFLAGS) -o $@ -c $<


# Python3 package
PYTHON=  python3
PACKAGE= _core.abi3.$(SOEXT)
OBJS=    src/wrapper.o

.PHONY: package
package: mulder/$(PACKAGE)

mulder/$(PACKAGE): setup.py src/build-core.py $(OBJS) lib/$(LIB)
	$(PYTHON) setup.py build --build-lib .

src/%.o: src/%.c src/%.h
	$(CC) $(LIB_CFLAGS) -c -o $@ $<


# Examples
.PHONY: examples
examples: bin/test

EXAMPLES_CFLAGS= $(CFLAGS) -Isrc -Llib $(RPATH)

bin/%: examples/%.c | lib/$(LIB) bindir
	$(CC) $(EXAMPLES_CFLAGS) -o $@ $^ -lmulder

.PHONY: bindir
bindir:
	@mkdir -p bin


# Cleaning
.PHONY: clean
clean:
	rm -rf build
	rm -rf lib
	rm -f src/*.o
	rm -rf mulder/$(PACKAGE) mulder/__pycache__
