/* Error handling */
const char * mulder_error_get(void);
void mulder_error_clear(void);

/* Return codes */
enum mulder_return {
    MULDER_SUCCESS = 0,
    MULDER_FAILURE
};

/* Vectorized layer height */
void mulder_layer_height_v(
    const struct mulder_layer * layer,
    int n,
    const double * position,
    double * height
);

/* Vectorized layer gradient */
void mulder_layer_gradient_v(
    const struct mulder_layer * layer,
    int n,
    const double * position,
    double * gradient
);

/* Vectorized geographic coordinates */
void mulder_layer_coordinates_v(
    const struct mulder_layer * layer,
    int n,
    const double * map_position,
    double * geographic_position
);

/* Vectorized map projection */
void mulder_layer_project_v(
    const struct mulder_layer * layer,
    int n,
    const double * geographic_position,
    double * map_position
);

/* Vectorized geomagnetic field */
void mulder_geomagnet_field_v(
    struct mulder_geomagnet * geomagnet,
    int n,
    const double * position,
    double * field
);

/* Vectorized flux computation */
enum mulder_return mulder_fluxmeter_flux_v(
    struct mulder_fluxmeter * fluxmeter,
    const double * position,
    const double * direction,
    int size,
    const double * energy,
    double * result
);

/* Vectorized reference flux */
void mulder_reference_flux_v(
    struct mulder_reference * reference,
    double height,
    double elevation,
    int n,
    const double * energy,
    double * flux
);

/* Vectorized state flux */
void mulder_state_flux_v(
    struct mulder_reference * reference,
    int n,
    const int * pid,
    const double * data,
    double * flux
);

/* Vectorized transport */
enum mulder_return mulder_fluxmeter_transport_v(
    struct mulder_fluxmeter * fluxmeter,
    int n,
    const int * pid_in,
    const double * data_in,
    int * pid_out,
    double * data_out
);

/* Vectorized intersections */
enum mulder_return mulder_fluxmeter_intersect_v(
    struct mulder_fluxmeter * fluxmeter,
    const double * position,
    int n,
    const double * direction,
    int * layer,
    double * intersection
);

/* Vectorized gramage */
enum mulder_return mulder_fluxmeter_grammage_v(
    struct mulder_fluxmeter * fluxmeter,
    const double * position,
    int n,
    const double * direction,
    double * grammage
);

/* Vectorized locator */
enum mulder_return mulder_fluxmeter_whereami_v(
    struct mulder_fluxmeter * fluxmeter,
    int n,
    const double * position,
    int * layer
);

/* Vectorized pseudo-random numbers */
void mulder_prng_uniform01_v(
    struct mulder_prng * prng,
    int n,
    double * values
);

/* Create a Turtle map from raw data */
enum mulder_return mulder_map_create(
    const char * path,
    const char * projection,
    int nx,
    int ny,
    double xmin,
    double xmax,
    double ymin,
    double ymax,
    const double * z
);

/* Generate physics tables for Pumas */
enum mulder_return mulder_generate_physics(
    const char * path,
    const char * destination,
    const char * dump
);
