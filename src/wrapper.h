/* Error handling */
const char * mulder_error_get(void);
void mulder_error_clear(void);

/* Return codes */
enum mulder_return {
    MULDER_SUCCESS = 0,
    MULDER_FAILURE
};

/* Vectorized layer height */
enum mulder_return mulder_layer_height_v(
    const struct mulder_layer * layer,
    int nx,
    int ny,
    const double * x,
    const double * y,
    double * z);

/* Vectorized layer gradient */
enum mulder_return mulder_layer_gradient_v(
    const struct mulder_layer * layer,
    int n,
    const double * x,
    const double * y,
    double * gx,
    double * gy);

/* Vectorized geodetic coordinates */
enum mulder_return mulder_layer_geodetic_v(
    const struct mulder_layer * layer,
    int n,
    const double * x,
    const double * y,
    double * latitude,
    double * longitude);

/* Vectorized map coordinates */
enum mulder_return mulder_layer_coordinates_v(
    const struct mulder_layer * layer,
    int n,
    const double * latitude,
    const double * longitude,
    double * x,
    double * y);

/* Vectorized flux computation */
enum mulder_return mulder_fluxmeter_flux_v(
    struct mulder_fluxmeter * fluxmeter,
    double latitude,
    double longitude,
    double height,
    double azimuth,
    double elevation,
    int n,
    const double * energy,
    double * result);

/* Vectorized reference flux */
enum mulder_return mulder_reference_flux_v(
    struct mulder_reference * reference,
    enum mulder_selection selection,
    double height,
    double elevation,
    int n,
    const double * energy,
    double * flux);

/* Vectorized intersections */
enum mulder_return mulder_fluxmeter_intersect_v(
    struct mulder_fluxmeter * fluxmeter,
    double latitude,
    double longitude,
    double height,
    int n,
    const double * azimuth,
    const double * elevation,
    int * layer,
    double * x,
    double * y,
    double * z);

/* Vectorized gramage */
enum mulder_return mulder_fluxmeter_grammage_v(
    struct mulder_fluxmeter * fluxmeter,
    double latitude,
    double longitude,
    double height,
    int n,
    const double * azimuth,
    const double * elevation,
    double * grammage);

/* Vectorized locator */
enum mulder_return mulder_fluxmeter_whereami_v(
    struct mulder_fluxmeter * fluxmeter,
    int n,
    const double * latitude,
    const double * longitude,
    const double * height,
    int * layer);

/* Vectorized pseudo-random numbers */
void mulder_prng_uniform01_v(
    struct mulder_prng * prng,
    int n,
    double * values);

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
    const double * z);

/* Generate physics tables for Pumas */
enum mulder_return mulder_generate_physics(
    const char * path,
    const char * destination,
    const char * dump);
