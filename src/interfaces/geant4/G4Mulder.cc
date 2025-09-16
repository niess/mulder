// Geant4 interface.
#include "G4Navigator.hh"
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

namespace G4Mulder{
    struct GeometryDefinition: public mulder_geometry_definition {
        GeometryDefinition(const G4VPhysicalVolume * world);
        ~GeometryDefinition() {};

        size_t GetMediumIndex(const G4VPhysicalVolume * volume) const;
        const G4VPhysicalVolume * GetWorld() const;

        std::vector<const G4Material *> materials;
        std::vector<const G4VPhysicalVolume *> volumes;
        std::unordered_map<const G4VPhysicalVolume *, size_t> mediaIndices;
    };

    struct MaterialDefinition: public mulder_material_definition {
        MaterialDefinition(const G4Material * material);
        ~MaterialDefinition() {};

        const G4Material * g4Material;
    };

    struct GeometryMedium: public mulder_geometry_medium {
        GeometryMedium(const G4VPhysicalVolume * volume);
        ~GeometryMedium() {};

        const G4VPhysicalVolume * g4Volume;
    };

    struct WeightedElement: public mulder_weighted_element {
        WeightedElement(const G4Element * element, double weight);
        ~WeightedElement() {};

        const G4Element * g4Element;
        std::string name;
        double molarWeight;
    };

    struct GeometryTracer: public mulder_geometry_tracer {
        GeometryTracer(const GeometryDefinition * definition);
        ~GeometryTracer();

        // Geometry data.
        const GeometryDefinition * definition;

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
// Implementation of Mulder C interface.
//
// ============================================================================

static struct mulder_geometry_definition * interface_definition(void) {
    auto && topVolume = G4Mulder::NewGeometry();
    return new G4Mulder::GeometryDefinition(topVolume);
}

static struct mulder_geometry_tracer * interface_tracer(
    const struct mulder_geometry_definition * definition_) {
    auto definition = (G4Mulder::GeometryDefinition *)definition_;
    return new G4Mulder::GeometryTracer(definition);
}

extern "C" struct mulder_interface G4MULDER_INITIALISE (void) {
    struct mulder_interface interface;
    interface.definition = &interface_definition;
    interface.tracer = &interface_tracer;
    return interface;
}

// ============================================================================
//
// Implementation of geometry definition.
//
// ============================================================================

static void geometry_destroy(struct mulder_geometry_definition * self) {
    auto geometry = (G4Mulder::GeometryDefinition *)self;
    G4Mulder::DropGeometry(geometry->GetWorld());
    delete geometry;
}

static struct mulder_material_definition * geometry_get_material(
    const struct mulder_geometry_definition * self,
    size_t index
){
    auto geometry = (G4Mulder::GeometryDefinition *)self;
    return new G4Mulder::MaterialDefinition(geometry->materials.at(index));
}

static struct mulder_geometry_medium * geometry_get_medium(
    const struct mulder_geometry_definition * self,
    size_t index
){
    auto geometry = (G4Mulder::GeometryDefinition *)self;
    return new G4Mulder::GeometryMedium(geometry->volumes.at(index));
}

static size_t geometry_materials_len(
    const struct mulder_geometry_definition * self
){
    auto geometry = (G4Mulder::GeometryDefinition *)self;
    return geometry->materials.size();
}

static size_t geometry_media_len(
const struct mulder_geometry_definition * self)
{
    auto geometry = (G4Mulder::GeometryDefinition *)self;
    return geometry->volumes.size();
}

static void append(
    std::vector<const G4Material *> &materials,
    std::vector<const G4VPhysicalVolume *> &volumes,
    std::unordered_map<const G4Material *, size_t> &materialsIndices,
    std::unordered_map<const G4VPhysicalVolume *, size_t> &mediaIndices,
    const G4VPhysicalVolume * current
){
    if (mediaIndices.count(current) == 0) {
        size_t n = volumes.size();
        mediaIndices.insert({current, n});
        volumes.push_back(current);

        auto && material = current->GetLogicalVolume()->GetMaterial();
        if (materialsIndices.count(material) == 0) {
            size_t m = materials.size();
            materialsIndices.insert({material, m});
            materials.push_back(material);
        }
    }
    auto && logical = current->GetLogicalVolume();
    G4int n = logical->GetNoDaughters();
    for (G4int i = 0; i < n; i++) {
        auto && volume = logical->GetDaughter(i);
        append(materials, volumes, materialsIndices, mediaIndices, volume);
    }
}

G4Mulder::GeometryDefinition::GeometryDefinition(
    const G4VPhysicalVolume * world)
{
    // Set interface.
    this->destroy = &geometry_destroy;
    this->material = &geometry_get_material;
    this->materials_len = &geometry_materials_len;
    this->medium = &geometry_get_medium;
    this->media_len = &geometry_media_len;


    // Scan volumes hierarchy.
    std::unordered_map<const G4Material *, size_t> materialsIndices;
    append(
        this->materials,
        this->volumes,
        materialsIndices,
        this->mediaIndices,
        world
    );
}

size_t G4Mulder::GeometryDefinition::GetMediumIndex(
    const G4VPhysicalVolume * volume) const
{
    try {
        return this->mediaIndices.at(volume);
    } catch (...) {
        return this->mediaIndices.size();
    }
}

const G4VPhysicalVolume * G4Mulder::GeometryDefinition::GetWorld() const {
    return (this->volumes.size() > 0) ?
        this->volumes[0] :
        nullptr;
}

// ============================================================================
//
// Implementation of material definition.
//
// ============================================================================

static void material_destroy(struct mulder_material_definition * self) {
    auto material = (G4Mulder::MaterialDefinition *)self;
    delete material;
}

static double material_density(
    const struct mulder_material_definition * self
){
    auto material = (G4Mulder::MaterialDefinition *)self;
    return material->g4Material->GetDensity() * (CLHEP::m3 / CLHEP::kg);
}

static struct mulder_weighted_element * material_get_element(
    const struct mulder_material_definition * self,
    size_t index
){
    auto material = ((G4Mulder::MaterialDefinition *)self)->g4Material;
    auto element = material->GetElement(index);
    double weight = double (
        material->GetVecNbOfAtomsPerVolume()[index] /
        material->GetTotNbOfAtomsPerVolume()
    );
    return new G4Mulder::WeightedElement(element, weight);
}

static size_t material_elements_len(
    const struct mulder_material_definition * self
){
    auto material = (G4Mulder::MaterialDefinition *)self;
    return material->g4Material->GetNumberOfElements();
}

static double material_I(
    const struct mulder_material_definition * self
){
    auto material = (G4Mulder::MaterialDefinition *)self;
    return material->g4Material->GetIonisation()->GetMeanExcitationEnergy() /
        CLHEP::GeV;
}

static const char * material_name(
    const struct mulder_material_definition * self
){
    auto material = (G4Mulder::MaterialDefinition *)self;
    return material->g4Material->GetName().c_str();
}

G4Mulder::MaterialDefinition::MaterialDefinition(
    const G4Material * material
):
    g4Material(material)
{
    // Set interface.
    this->destroy = &material_destroy;
    this->density = &material_density;
    this->elements_len = &material_elements_len;
    this->element = &material_get_element;
    this->I = (material->GetIonisation() == nullptr) ?
        nullptr : &material_I;
    this->name = &material_name;
}

// ============================================================================
//
// Implementation of weighted element.
//
// ============================================================================

static void element_destroy(struct mulder_weighted_element * self) {
    auto element = (G4Mulder::WeightedElement *)self;
    delete element;
}

static int element_Z(
    const struct mulder_weighted_element * self
){
    auto element = (G4Mulder::WeightedElement *)self;
    return int(element->g4Element->GetZ());
}

static double element_A(
    const struct mulder_weighted_element * self
){
    auto element = (G4Mulder::WeightedElement *)self;
    return element->g4Element->GetA() * (CLHEP::mole / CLHEP::g);
}

static double element_I(
    const struct mulder_weighted_element * self
){
    auto element = (G4Mulder::WeightedElement *)self;
    return element->g4Element->GetIonisation()->GetMeanExcitationEnergy() /
        CLHEP::GeV;
}

static const char * element_symbol(
    const struct mulder_weighted_element * self
){
    auto element = (G4Mulder::WeightedElement *)self;
    return element->name.c_str();
}

static double element_weight(
    const struct mulder_weighted_element * self
){
    auto element = (G4Mulder::WeightedElement *)self;
    return element->molarWeight;
}

G4Mulder::WeightedElement::WeightedElement(
    const G4Element * element, double weight_
):
    g4Element(element), molarWeight(weight_)
{
    this->name = element->GetSymbol();
    if (this->name.rfind("G4_", 0) != 0) {
        this->name = "G4_" + this->name;
    }

    // Set interface.
    this->destroy = &element_destroy;
    this->Z = &element_Z;
    this->A = &element_A;
    this->I = &element_I;
    this->symbol = &element_symbol;
    this->weight = &element_weight;
}

// ============================================================================
//
// Implementation of geometry medium.
//
// ============================================================================

static void medium_destroy(struct mulder_geometry_medium * self) {
    auto medium = (G4Mulder::GeometryMedium *)self;
    delete medium;
}

static const char * medium_material(
    const struct mulder_geometry_medium * self
){
    auto medium = (G4Mulder::GeometryMedium *)self;
    return medium->g4Volume->GetLogicalVolume()->GetMaterial()->GetName().c_str();
}

static const char * medium_description(
    const struct mulder_geometry_medium * self
){
    auto medium = (G4Mulder::GeometryMedium *)self;
    return medium->g4Volume->GetName().c_str();
}

G4Mulder::GeometryMedium::GeometryMedium(const G4VPhysicalVolume * volume):
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
// Implementation of geometry tracer.
//
// ============================================================================

static void tracer_destroy(struct mulder_geometry_tracer * self) {
    auto tracer = (G4Mulder::GeometryTracer *)self;
    delete tracer;
}

static size_t tracer_locate(
    struct mulder_geometry_tracer * self,
    struct mulder_vec3 position_
){
    auto tracer = (G4Mulder::GeometryTracer *)self;

    auto position = G4ThreeVector(
        position_.x * CLHEP::m,
        position_.y * CLHEP::m,
        position_.z * CLHEP::m
    );
    auto volume = tracer->navigator.LocateGlobalPointAndSetup(position);

    return tracer->definition->GetMediumIndex(volume);
}

static void tracer_reset(
    struct mulder_geometry_tracer * self,
    struct mulder_vec3 position,
    struct mulder_vec3 direction
){
    auto tracer = (G4Mulder::GeometryTracer *)self;

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
    struct mulder_geometry_tracer * self,
    double max_length
){
    auto tracer = (G4Mulder::GeometryTracer *)self;

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
    struct mulder_geometry_tracer * self,
    double length
){
    auto tracer = (G4Mulder::GeometryTracer *)self;

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
        auto geometry = (const G4Mulder::GeometryDefinition *)tracer->definition;
        tracer->currentIndex = geometry->GetMediumIndex(
            tracer->history->GetVolume()
        );
    }

    tracer->stepLength -= length;
    tracer->stepSafety -= length;
}

static void tracer_turn(
    struct mulder_geometry_tracer * self,
    struct mulder_vec3 direction
){
    auto tracer = (G4Mulder::GeometryTracer *)self;
    tracer->currentDirection = G4ThreeVector(
        direction.x,
        direction.y,
        direction.z
    );
}

static size_t tracer_medium(struct mulder_geometry_tracer * self){
    auto tracer = (G4Mulder::GeometryTracer *)self;
    return tracer->currentIndex;
}

static struct mulder_vec3 tracer_position(
    struct mulder_geometry_tracer * self
){
    auto tracer = (G4Mulder::GeometryTracer *)self;
    auto && r = tracer->currentPosition;
    return {
        r[0] / CLHEP::m,
        r[1] / CLHEP::m,
        r[2] / CLHEP::m
    };
}

G4Mulder::GeometryTracer::GeometryTracer(
    const G4Mulder::GeometryDefinition * definition_
): definition(definition_) {
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

G4Mulder::GeometryTracer::~GeometryTracer() {
    delete this->history;
}
