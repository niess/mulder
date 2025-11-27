G4MULDER_DIR=$(shell python3 -m mulder --prefix)/interface/geant4

CFLAGS= -O0 -g \
        -I$(G4MULDER_DIR) \
        $(shell geant4-config --cflags)

LIBS= $(shell geant4-config --libs)

module.so: module.cpp
	$(CXX) $(CFLAGS) -shared -fPIC -o $@ $< $(G4MULDER_DIR)/G4Mulder.cc $(LIBS)

clean:
	rm -f module.so
