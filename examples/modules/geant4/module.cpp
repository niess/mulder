// Geant4 interface.
#include "G4Box.hh"
#include "G4LogicalVolume.hh"
#include "G4Material.hh"
#include "G4NistManager.hh"
#include "G4PVPlacement.hh"
#include "G4VUserDetectorConstruction.hh"
// Mulder interface.
#include "G4Mulder.hh"


// ============================================================================
//
// Geant4 geometry implementation.
//
// ============================================================================

// Hard coded size parameters.
static const G4double WORLD_SIZE = 2.0 * CLHEP::km;
static const G4double DETECTOR_WIDTH = 20.0 * CLHEP::m;
static const G4double DETECTOR_HEIGHT = 10.0 * CLHEP::m;
static const G4double DETECTOR_OFFSET = 5.0 * CLHEP::cm;


struct DetectorConstruction: public G4VUserDetectorConstruction {
    G4VPhysicalVolume * Construct() {
        auto manager = G4NistManager::Instance();

        // World wolume, containing the atmosphere layer.
        G4LogicalVolume * world;
        {
            std::string name = "Atmosphere";
            auto solid = new G4Box(
                name,
                0.5 * WORLD_SIZE,
                0.5 * WORLD_SIZE,
                0.5 * WORLD_SIZE
            );
            auto material = manager->FindOrBuildMaterial("G4_AIR");
            world = new G4LogicalVolume(solid, material, name);
        }

        // Ground volume.
        {
            std::string name = "Soil";
            auto solid = new G4Box(
                name,
                0.5 * WORLD_SIZE,
                0.5 * WORLD_SIZE,
                0.25 * WORLD_SIZE
            );
            auto material = manager->FindOrBuildMaterial(
                "G4_CALCIUM_CARBONATE");
            auto volume = new G4LogicalVolume(solid, material, name);
            new G4PVPlacement(
                nullptr,
                G4ThreeVector(0.0, 0.0, -0.25 * WORLD_SIZE),
                volume,
                name,
                world,
                false,
                0
            );
        }

        // Collection volume.
        {
            std::string name = "Detector";
            auto solid = new G4Box(
                name,
                0.5 * DETECTOR_WIDTH,
                0.5 * DETECTOR_WIDTH,
                0.5 * DETECTOR_HEIGHT
            );
            auto material = manager->FindOrBuildMaterial("G4_AIR");
            auto volume = new G4LogicalVolume(solid, material, name);
            new G4PVPlacement(
                nullptr,
                G4ThreeVector(
                    0.0, 0.0, 0.5 * DETECTOR_HEIGHT + DETECTOR_OFFSET),
                volume,
                name,
                world,
                false,
                0
            );
        }

        return new G4PVPlacement(
            nullptr,
            G4ThreeVector(0.0, 0.0, 0.0),
            world,
            world->GetName(),
            nullptr,
            false,
            0
        );
    }
};


// ============================================================================
//
// Mulder hooks.
//
// ============================================================================

const G4VPhysicalVolume * G4Mulder::NewGeometry() {
    // Build the geometry and return the top "World" volume.
    return DetectorConstruction().Construct();
}

void G4Mulder::DropGeometry(const G4VPhysicalVolume * volume) {
    // Delete any sub-volume(s).
    auto && logical = volume->GetLogicalVolume();
    while (logical->GetNoDaughters()) {
        auto daughter = logical->GetDaughter(0);
        logical->RemoveDaughter(daughter);
        G4Mulder::DropGeometry(daughter);
    }
    // Delete this volume.
    delete logical->GetSolid();
    delete logical;
    delete volume;
}
