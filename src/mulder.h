#ifndef mulder_h
#define mulder_h
#ifdef __cplusplus
extern "C" {
#endif


/* Version macros */
#define MULDER_VERSION_MAJOR 0
#define MULDER_VERSION_MINOR 1
#define MULDER_VERSION_PATCH 0


/* Library error handler (default implementation, mutable) */
extern void (*mulder_error)(const char * message);


/* Topographic layer (semi-opaque structure) */
struct mulder_layer {
    /* Initial settings (non-mutable) */
    const char * const material;
    const char * const model;
    const double offset;

    /* Mutable propertie(s) */
    double density;

    /* Map metadata */
    const char * const encoding;
    const char * const projection;
    const int nx;
    const int ny;
    const double xmin;
    const double xmax;
    const double ymin;
    const double ymax;
    const double zmin; /* including offset */
    const double zmax; /* including offset */
};

struct mulder_layer * mulder_layer_create(
    const char * material,
    const char * model,
    double offset);

void mulder_layer_destroy(struct mulder_layer ** layer);

double mulder_layer_height(
    const struct mulder_layer * layer,
    double x,
    double y);

void mulder_layer_gradient(
    const struct mulder_layer * layer,
    double x,
    double y,
    double * gx,
    double * gy);

void mulder_layer_geodetic(
    const struct mulder_layer * layer,
    double x,
    double y,
    double * latitude,
    double * longitude);

void mulder_layer_coordinates(
    const struct mulder_layer * layer,
    double latitude,
    double longitude,
    double * x,
    double * y);


/* Reference (opensky) muon flux model */
struct mulder_reference {
    double height_min;
    double height_max;
    double (*flux)(struct mulder_reference * reference,
                   double height,
                   double elevation,
                   double kinetic_energy);
};

struct mulder_reference * mulder_reference_load_table(const char * path);
void mulder_reference_destroy_table(struct mulder_reference ** reference);


/* Transport modes for muon flux computations */
enum mulder_mode {
    /* Muons are transported using a deterministic CSDA. This is the default
     * mode of operation
     */
    MULDER_CSDA = 0,
    /* As previously, but catastrophic energy losses are randomised,
     * e.g. as in MUM (Sokalski, Bugaev and Klimushin, hep-ph/0010322)
     */
    MULDER_MIXED,
    /* A detailed Monte Carlo simulation is done, including multiple
     * scattering
     */
    MULDER_DETAILED
};


/* Muon flux calculator (semi-opaque structure) */
struct mulder_fluxmeter {
    /* Initial settings (non mutable) */
    const char * const physics;
    const int size;
    const struct mulder_layer ** layers;

    /* Mutable properties */
    enum mulder_mode mode;
    struct mulder_reference * reference;
};

struct mulder_fluxmeter * mulder_fluxmeter_create(
    const char * physics,
    int size,
    struct mulder_layer * layers[]);

void mulder_fluxmeter_destroy(struct mulder_fluxmeter ** fluxmeter);

int mulder_fluxmeter_intersect(
    struct mulder_fluxmeter * fluxmeter,
    double * latitude,
    double * longitude,
    double * height,
    double azimuth,
    double elevation);

double mulder_fluxmeter_flux(
    struct mulder_fluxmeter * fluxmeter,
    double kinetic_energy,
    double latitude,
    double longitude,
    double height,
    double azimuth,
    double elevation);

double mulder_fluxmeter_grammage(
    struct mulder_fluxmeter * fluxmeter,
    double latitude,
    double longitude,
    double height,
    double azimuth,
    double elevation,
    double * grammage);

int mulder_fluxmeter_whereami(
    struct mulder_fluxmeter * fluxmeter,
    double latitude,
    double longitude,
    double height);


#ifdef __cplusplus
extern }
#endif
#endif
