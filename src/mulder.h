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


/* Observation position, using geographic coordinates (GPS-like) */
struct mulder_position {
    double latitude;  /* deg */
    double longitude; /* deg */
    double height;    /* m, w.r.t. WGS84 ellipsoid */
};


/* Observation direction, using Horizontal coordinates */
struct mulder_direction {
    double azimuth;   /* deg, w.r.t. geographic North */
    double elevation; /* deg, w.r.t. the local horizontal */
};


/* Projected (map) coordinates */
struct mulder_projection {
    double x;
    double y;
};


/* East, North, Upward (ENU) coordinates (vector like) */
struct mulder_enu {
    double east;
    double north;
    double upward;
};


/* Topographic layer (semi-opaque structure) */
struct mulder_layer {
    /* Initial settings (non-mutable) */
    const char * const material;
    const char * const model;
    const double offset; /* m */

    /* Mutable propertie(s) */
    double density;      /* kg / m^3 */

    /* Map metadata */
    const char * const encoding;
    const char * const projection;
    const int nx;
    const int ny;
    const double xmin;
    const double xmax;
    const double ymin;
    const double ymax;
    const double zmin; /* m, including offset */
    const double zmax; /* m, including offset */
};

struct mulder_layer * mulder_layer_create(
    const char * material,
    const char * model,
    double offset
);

void mulder_layer_destroy(struct mulder_layer ** layer);

double mulder_layer_height(
    const struct mulder_layer * layer,
    struct mulder_projection projection
);

struct mulder_projection mulder_layer_gradient(
    const struct mulder_layer * layer,
    struct mulder_projection projection
);

struct mulder_position mulder_layer_position(
    const struct mulder_layer * layer,
    struct mulder_projection projection
);

struct mulder_projection mulder_layer_project(
    const struct mulder_layer * layer,
    struct mulder_position position
);


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
    int year
);

void mulder_geomagnet_destroy(struct mulder_geomagnet ** geomagnet);

struct mulder_enu mulder_geomagnet_field(
    const struct mulder_geomagnet * geomagnet,
    struct mulder_position position
);

/* Container for atmosphere local properties */
struct mulder_atmosphere {
    double density;
    double gradient;
};

/* Stratified Earth geometry */
struct mulder_geometry {
    /* Mutable properties */
    struct mulder_atmosphere (*atmosphere)(double height);
    struct mulder_geomagnet * geomagnet;

    /* Initial settings (non mutable) */
    const int size;
    struct mulder_layer * const layers[];
};

struct mulder_geometry * mulder_geometry_create(
    int size,
    struct mulder_layer * layers[]
);

void mulder_geometry_destroy(struct mulder_geometry ** geometry);


/* Particles identifiers (PDG numbering scheme) */
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
        double kinetic_energy
    );
};

struct mulder_reference * mulder_reference_default(void);

struct mulder_reference * mulder_reference_load_table(const char * path);

void mulder_reference_destroy_table(struct mulder_reference ** reference);


/* Memory layout for Pseudo Random Numbers Generators (PRNGs) */
struct mulder_prng {
    unsigned long (*get_seed)(struct mulder_prng * prng);

    void (*set_seed)(
        struct mulder_prng * prng,
        const unsigned long * seed
    );

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
    struct mulder_geometry * const geometry;

    /* Mutable properties */
    enum mulder_mode mode;
    struct mulder_prng * prng;
    struct mulder_reference * reference;
};

struct mulder_fluxmeter * mulder_fluxmeter_create(
    const char * physics,
    struct mulder_geometry * geometry
);

void mulder_fluxmeter_destroy(struct mulder_fluxmeter ** fluxmeter);


/* Observation state */
struct mulder_state {
    /* Particle identifier */
    enum mulder_pid pid;
    /* Location */
    struct mulder_position position;
    /* Observation direction */
    struct mulder_direction direction;
    /* Kinetic energy, in GeV */
    double energy;
    /* Transport weight (unused on input) */
    double weight;
};


/* Muon flux computation */
struct mulder_flux mulder_fluxmeter_flux(
    struct mulder_fluxmeter * fluxmeter,
    struct mulder_state state
);


/* Monte Carlo interface */
struct mulder_flux mulder_state_flux( /* sample reference flux */
    struct mulder_state state,
    struct mulder_reference * reference
);

struct mulder_state mulder_fluxmeter_transport( /* transport state */
    struct mulder_fluxmeter * fluxmeter,
    struct mulder_state state
);


/* Geometry related utilities */
struct mulder_intersection {
    int layer;
    struct mulder_position position;
};

struct mulder_intersection mulder_fluxmeter_intersect(
    struct mulder_fluxmeter * fluxmeter,
    struct mulder_position position,
    struct mulder_direction direction
);

double mulder_fluxmeter_grammage(
    struct mulder_fluxmeter * fluxmeter,
    struct mulder_position position,
    struct mulder_direction direction,
    double * grammage
);

int mulder_fluxmeter_whereami(
    struct mulder_fluxmeter * fluxmeter,
    struct mulder_position position
);


#ifdef __cplusplus
extern }
#endif
#endif
