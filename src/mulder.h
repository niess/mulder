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


/* Geomagnetic field (semi-opaque structure) */
struct mulder_geomagnet {
    /* Initial settings (non-mutable) */
    const char * const model;
    const int day;
    const int month;
    const int year;

    /* Model metadata */
    const int order;
    const double height_min;
    const double height_max;
};

struct mulder_geomagnet * mulder_geomagnet_create(
    const char * model,
    int day,
    int month,
    int year);

void mulder_geomagnet_destroy(struct mulder_geomagnet ** geomagnet);

void mulder_geomagnet_field(
    const struct mulder_geomagnet * geomagnet,
    double latitude,
    double longitude,
    double height,
    double * east,
    double * north,
    double * upward);


/* Particles identifiers (PDG nubering scheme) */
enum mulder_pid {
    MULDER_ANY = 0,
    MULDER_MUON = 13,
    MULDER_ANTIMUON = -13
};


/* Container for muon flux data */
struct mulder_flux {
    double value;
    double asymmetry; /* charge asymmetry */
};


/* Reference (opensky) muon flux model */
struct mulder_reference {
    double energy_min;
    double energy_max;
    double height_min;
    double height_max;
    struct mulder_flux (*flux)(
        struct mulder_reference * reference,
        double height,
        double elevation,
        double kinetic_energy);
};

struct mulder_reference * mulder_reference_default(void);

struct mulder_reference * mulder_reference_load_table(const char * path);
void mulder_reference_destroy_table(struct mulder_reference ** reference);


/* Memory layout for Pseudo Random Numbers Generators (PRNGs) */
struct mulder_prng {
    unsigned long (*get_seed)(struct mulder_prng * prng);

    void (*set_seed)(struct mulder_prng * prng,
                     const unsigned long * seed);

    double (*uniform01)(struct mulder_prng * prng); /* Mandatory */
};


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
    enum mulder_pid selection;
    struct mulder_prng * prng;
    struct mulder_reference * reference;
    struct mulder_geomagnet * geomagnet;
};

struct mulder_fluxmeter * mulder_fluxmeter_create(
    const char * physics,
    int size,
    struct mulder_layer * layers[]);

void mulder_fluxmeter_destroy(struct mulder_fluxmeter ** fluxmeter);


/* Muon flux computation */
struct mulder_flux mulder_fluxmeter_flux(
    struct mulder_fluxmeter * fluxmeter,
    double kinetic_energy,
    double latitude,
    double longitude,
    double height,
    double azimuth,
    double elevation);


/* Monte Carlo interface */
struct mulder_state {
    /* Particle identifier */
    enum mulder_pid pid;
    /* Location */
    double latitude;
    double longitude;
    double height;
    /* Observation direction */
    double azimuth;
    double elevation;
    /* Kinetic energy, in GeV */
    double energy;
    /* Transport weight (unused on input) */
    double weight;
};

struct mulder_flux mulder_state_flux( /* sample reference flux */
    const struct mulder_state * state,
    struct mulder_reference * reference);

struct mulder_state mulder_fluxmeter_transport( /* transport state */
    struct mulder_fluxmeter * fluxmeter,
    const struct mulder_state * state);


/* Geometry related utilities */
int mulder_fluxmeter_intersect(
    struct mulder_fluxmeter * fluxmeter,
    double * latitude,
    double * longitude,
    double * height,
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
