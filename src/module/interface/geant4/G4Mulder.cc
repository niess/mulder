// Geant4 interface.
#include "G4Navigator.hh"
#include "G4NistManager.hh"
#include "G4Material.hh"
#include "G4VPhysicalVolume.hh"
// Mulder C interface.
#include "mulder.h"
// Mulder Geant4 interface.
#include "G4Mulder.hh"
// C++ standard library.
#include <unordered_map>

// Entry point for Mulder.
#ifndef G4MULDER_INITIALISE
#define G4MULDER_INITIALISE mulder_initialise
#endif


// ============================================================================
//
// Local interface, bridging Geant4 and Mulder.
//
// ============================================================================

namespace G4Mulder {
    struct Component: public mulder_component {
        Component(const G4Element * element, double weight);
        ~Component() {};

        std::string name;
        double molarWeight;
    };

    struct Element: public mulder_element {
        Element(const G4Element * element);
        ~Element() {};

        const G4Element * g4Element;
    };

    struct Geometry: public mulder_geometry {
        Geometry(const G4VPhysicalVolume * world);
        ~Geometry() {};

        size_t GetMediumIndex(const G4VPhysicalVolume * volume) const;
        const G4VPhysicalVolume * GetWorld() const;

        std::vector<const G4VPhysicalVolume *> volumes;
        std::unordered_map<const G4VPhysicalVolume *, size_t> mediaIndices;
    };

    struct Material: public mulder_material {
        Material(const G4Material * material);
        ~Material() {};

        const G4Material * g4Material;
    };

    struct Medium: public mulder_medium {
        Medium(const G4VPhysicalVolume * volume);
        ~Medium() {};

        const G4VPhysicalVolume * g4Volume;
    };

    struct Tracer: public mulder_tracer {
        Tracer(const Geometry * definition);
        ~Tracer();

        // Geometry data.
        const Geometry * definition;

        // State data.
        G4ThreeVector currentDirection;
        size_t currentIndex;
        G4ThreeVector currentPosition;
        double stepLength;
        double stepSafety;

        G4TouchableHistory * history;
        G4Navigator navigator;
    };
}

// ============================================================================
//
// Implementation of Mulder C module.
//
// ============================================================================

static struct mulder_element * module_element(const char * symbol) {
    auto element = G4Element::GetElement(symbol, false);
    if (element == nullptr) {
        std::string name = symbol;
        if (name.rfind("G4_", 0) == 0) {
            element = G4Element::GetElement(symbol + 3, false);
            if (element == nullptr) {
                auto nist = G4NistManager::Instance();
                element = nist->FindOrBuildElement(symbol + 3);
            }
        }
    }
    if (element == nullptr) {
        return nullptr;
    } else {
        return new G4Mulder::Element(element);
    }
}

static struct mulder_geometry * module_geometry(void) {
    auto && topVolume = G4Mulder::NewGeometry();
    return new G4Mulder::Geometry(topVolume);
}

static struct mulder_material * module_material(const char * name) {
    auto material = G4Material::GetMaterial(name, false);
    if (material == nullptr) {
        material = G4NistManager::Instance()->FindOrBuildMaterial(name);
    }
    if (material == nullptr) {
        return nullptr;
    } else {
        return new G4Mulder::Material(material);
    }
}

extern "C" struct mulder_module G4MULDER_INITIALISE (void) {
    struct mulder_module mod;
    mod.element = &module_element;
    mod.geometry = &module_geometry;
    mod.material = &module_material;
    return mod;
}

// ============================================================================
//
// Implementation of geometry object.
//
// ============================================================================

static void geometry_destroy(struct mulder_geometry * self) {
    auto geometry = (G4Mulder::Geometry *)self;
    G4Mulder::DropGeometry(geometry->GetWorld());
    delete geometry;
}

static struct mulder_medium * geometry_get_medium(
    const struct mulder_geometry * self,
    size_t index
){
    auto geometry = (G4Mulder::Geometry *)self;
    return new G4Mulder::Medium(geometry->volumes.at(index));
}

static size_t geometry_media_len(
const struct mulder_geometry * self)
{
    auto geometry = (G4Mulder::Geometry *)self;
    return geometry->volumes.size();
}

static struct mulder_tracer * geometry_tracer(
    const struct mulder_geometry * self) {
    auto geometry = (G4Mulder::Geometry *)self;
    return new G4Mulder::Tracer(geometry);
}

static void append(
    std::vector<const G4VPhysicalVolume *> &volumes,
    std::unordered_map<const G4VPhysicalVolume *, size_t> &mediaIndices,
    const G4VPhysicalVolume * current
){
    if (mediaIndices.count(current) == 0) {
        size_t n = volumes.size();
        mediaIndices.insert({current, n});
        volumes.push_back(current);
    }
    auto && logical = current->GetLogicalVolume();
    G4int n = logical->GetNoDaughters();
    for (G4int i = 0; i < n; i++) {
        auto && volume = logical->GetDaughter(i);
        append(volumes, mediaIndices, volume);
    }
}

G4Mulder::Geometry::Geometry(
    const G4VPhysicalVolume * world)
{
    // Set interface.
    this->destroy = &geometry_destroy;
    this->medium = &geometry_get_medium;
    this->media_len = &geometry_media_len;
    this->tracer = &geometry_tracer;

    // Scan volumes hierarchy.
    append(
        this->volumes,
        this->mediaIndices,
        world
    );
}

size_t G4Mulder::Geometry::GetMediumIndex(
    const G4VPhysicalVolume * volume) const
{
    try {
        return this->mediaIndices.at(volume);
    } catch (...) {
        return this->mediaIndices.size();
    }
}

const G4VPhysicalVolume * G4Mulder::Geometry::GetWorld() const {
    return (this->volumes.size() > 0) ?
        this->volumes[0] :
        nullptr;
}

// ============================================================================
//
// Implementation of material object.
//
// ============================================================================

static void material_destroy(struct mulder_material * self) {
    auto material = (G4Mulder::Material *)self;
    delete material;
}

static double material_density(
    const struct mulder_material * self
){
    auto material = (G4Mulder::Material *)self;
    return material->g4Material->GetDensity() * (CLHEP::m3 / CLHEP::kg);
}

static struct mulder_component * material_get_element(
    const struct mulder_material * self,
    size_t index
){
    auto material = ((G4Mulder::Material *)self)->g4Material;
    auto element = material->GetElement(index);
    double weight = double (
        material->GetVecNbOfAtomsPerVolume()[index] /
        material->GetTotNbOfAtomsPerVolume()
    );
    return new G4Mulder::Component(element, weight);
}

static size_t material_elements_len(
    const struct mulder_material * self
){
    auto material = (G4Mulder::Material *)self;
    return material->g4Material->GetNumberOfElements();
}

static double material_I(
    const struct mulder_material * self
){
    auto material = (G4Mulder::Material *)self;
    return material->g4Material->GetIonisation()->GetMeanExcitationEnergy() /
        CLHEP::GeV;
}

G4Mulder::Material::Material(
    const G4Material * material
):
    g4Material(material)
{
    // Set interface.
    this->destroy = &material_destroy;
    this->component = &material_get_element;
    this->components_len = &material_elements_len;
    this->density = &material_density;
    this->I = (material->GetIonisation() == nullptr) ?
        nullptr : &material_I;
}

// ============================================================================
//
// Implementation of component object.
//
// ============================================================================

static void component_destroy(struct mulder_component * self) {
    auto component = (G4Mulder::Component *)self;
    delete component;
}

static const char * component_symbol(const struct mulder_component * self) {
    auto component = (G4Mulder::Component *)self;
    return component->name.c_str();
}

static double component_weight(const struct mulder_component * self) {
    auto component = (G4Mulder::Component *)self;
    return component->molarWeight;
}

static bool is_nist(const G4Element * element) {
    auto nist = G4NistManager::Instance();
    auto Z = nist->GetZ(element->GetName());
    const double EPSILON = FLT_EPSILON;
    if (Z > 0) {
        auto dZ = element->GetZ() - Z;
        if (fabs(dZ) > EPSILON) return false;
        auto dA = element->GetAtomicMassAmu() - nist->GetAtomicMassAmu(Z);
        if (fabs(dA) > EPSILON) return false;
        auto dI = element->GetIonisation()->GetMeanExcitationEnergy() -
            nist->GetMeanIonisationEnergy(Z);
        return (fabs(dI) / CLHEP::eV <= EPSILON);
     } else {
         return false;
     }
}

G4Mulder::Component::Component(const G4Element * element, double weight_):
    molarWeight(weight_)
{
    this->name = element->GetName();
    if (is_nist(element)) {
        this->name = "G4_" + this->name;
    }

    // Set interface.
    this->destroy = &component_destroy;
    this->symbol = &component_symbol;
    this->weight = &component_weight;
}

// ============================================================================
//
// Implementation of element object.
//
// ============================================================================

static void element_destroy(struct mulder_element * self) {
    auto element = (G4Mulder::Element *)self;
    delete element;
}

static int element_Z(
    const struct mulder_element * self) {
    auto element = (G4Mulder::Element *)self;
    return int(element->g4Element->GetZ());
}

static double element_A(const struct mulder_element * self) {
    auto element = (G4Mulder::Element *)self;
    return element->g4Element->GetA() * (CLHEP::mole / CLHEP::g);
}

static double element_I(const struct mulder_element * self) {
    auto element = (G4Mulder::Element *)self;
    return element->g4Element->GetIonisation()->GetMeanExcitationEnergy() /
        CLHEP::GeV;
}

G4Mulder::Element::Element(const G4Element * element):
    g4Element(element)
{
    // Set interface.
    this->destroy = &element_destroy;
    this->Z = &element_Z;
    this->A = &element_A;
    this->I = &element_I;
}

// ============================================================================
//
// Implementation of medium object.
//
// ============================================================================

static void medium_destroy(struct mulder_medium * self) {
    auto medium = (G4Mulder::Medium *)self;
    delete medium;
}

static const char * medium_material(
    const struct mulder_medium * self
){
    auto medium = (G4Mulder::Medium *)self;
    return medium->g4Volume->GetLogicalVolume()->GetMaterial()->GetName().c_str();
}

static const char * medium_description(
    const struct mulder_medium * self
){
    auto medium = (G4Mulder::Medium *)self;
    return medium->g4Volume->GetName().c_str();
}

G4Mulder::Medium::Medium(const G4VPhysicalVolume * volume):
    g4Volume(volume)
{
    // Set interface.
    this->destroy = &medium_destroy;
    this->material = &medium_material;
    this->density = nullptr;
    this->description = &medium_description;
}

// ============================================================================
//
// Implementation of tracer object.
//
// ============================================================================

static void tracer_destroy(struct mulder_tracer * self) {
    auto tracer = (G4Mulder::Tracer *)self;
    delete tracer;
}

static size_t tracer_locate(
    struct mulder_tracer * self,
    struct mulder_vec3 position_
){
    auto tracer = (G4Mulder::Tracer *)self;

    auto position = G4ThreeVector(
        position_.x * CLHEP::m,
        position_.y * CLHEP::m,
        position_.z * CLHEP::m
    );
    auto volume = tracer->navigator.LocateGlobalPointAndSetup(position);

    return tracer->definition->GetMediumIndex(volume);
}

static void tracer_reset(
    struct mulder_tracer * self,
    struct mulder_vec3 position,
    struct mulder_vec3 direction
){
    auto tracer = (G4Mulder::Tracer *)self;

    // Reset Geant4 navigation.
    tracer->currentPosition = G4ThreeVector(
        position.x * CLHEP::m,
        position.y * CLHEP::m,
        position.z * CLHEP::m
    );

    tracer->currentDirection = G4ThreeVector(
        direction.x,
        direction.y,
        direction.z
    );

    tracer->navigator.ResetStackAndState();
    tracer->navigator.LocateGlobalPointAndUpdateTouchable(
        tracer->currentPosition,
        tracer->currentDirection,
        tracer->history,
        false // Do not use history.
    );

    // Reset internal state.
    tracer->currentIndex = tracer->definition->GetMediumIndex(
        tracer->history->GetVolume()
    );
    tracer->stepLength = 0.0;
    tracer->stepSafety = 0.0;
}

static double tracer_trace(
    struct mulder_tracer * self,
    double max_length
){
    auto tracer = (G4Mulder::Tracer *)self;

    G4double safety = 0.0;
    G4double s = tracer->navigator.ComputeStep(
        tracer->currentPosition,
        tracer->currentDirection,
        max_length * CLHEP::m,
        safety
    );
    double step = s / CLHEP::m;
    tracer->stepLength = step;
    tracer->stepSafety = safety / CLHEP::m;

    return (step < max_length) ? step : max_length;
}

static void tracer_move(
    struct mulder_tracer * self,
    double length
){
    auto tracer = (G4Mulder::Tracer *)self;

    tracer->currentPosition += (length * CLHEP::m) * tracer->currentDirection;

    if ((length > 0.0) && (length < tracer->stepSafety)) {
        tracer->navigator.LocateGlobalPointWithinVolume(
            tracer->currentPosition
        );
    } else {
        if (length >= tracer->stepLength) {
            tracer->navigator.SetGeometricallyLimitedStep();
        }
        tracer->navigator.LocateGlobalPointAndUpdateTouchable(
            tracer->currentPosition,
            tracer->currentDirection,
            tracer->history
        );
        auto geometry = (const G4Mulder::Geometry *)tracer->definition;
        tracer->currentIndex = geometry->GetMediumIndex(
            tracer->history->GetVolume()
        );
    }

    tracer->stepLength -= length;
    tracer->stepSafety -= length;
}

static void tracer_turn(
    struct mulder_tracer * self,
    struct mulder_vec3 direction
){
    auto tracer = (G4Mulder::Tracer *)self;
    tracer->currentDirection = G4ThreeVector(
        direction.x,
        direction.y,
        direction.z
    );
}

static size_t tracer_medium(struct mulder_tracer * self) {
    auto tracer = (G4Mulder::Tracer *)self;
    return tracer->currentIndex;
}

static struct mulder_vec3 tracer_position(struct mulder_tracer * self) {
    auto tracer = (G4Mulder::Tracer *)self;
    auto && r = tracer->currentPosition;
    return {
        r[0] / CLHEP::m,
        r[1] / CLHEP::m,
        r[2] / CLHEP::m
    };
}

G4Mulder::Tracer::Tracer(const G4Mulder::Geometry * definition_):
    definition(definition_) {
    // Initialise Geant4 navigator.
    this->navigator.SetWorldVolume(
        (G4VPhysicalVolume *) definition_->GetWorld());
    this->history = this->navigator.CreateTouchableHistory();

    // Initialise internal data.
    this->currentDirection = G4ThreeVector(0.0, 0.0, 1.0);
    this->currentIndex = 0;
    this->currentPosition = G4ThreeVector(0.0, 0.0, 0.0);
    this->stepLength = 0.0;
    this->stepSafety = 0.0;

    // Set C interface.
    this->destroy = &tracer_destroy;
    this->locate = &tracer_locate;
    this->reset = &tracer_reset;
    this->trace = &tracer_trace;
    this->move = &tracer_move;
    this->turn = &tracer_turn;
    this->medium = &tracer_medium;
    this->position = &tracer_position;
}

G4Mulder::Tracer::~Tracer() {
    delete this->history;
}
