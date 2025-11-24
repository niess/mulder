#ifndef G4Mulder_hh
#define G4Mulder_hh

class G4VPhysicalVolume;

namespace G4Mulder {
    const G4VPhysicalVolume * NewGeometry();
    void DropGeometry(const G4VPhysicalVolume * volume);
}

#endif
