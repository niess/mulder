#! /bin/bash

TAG="${1:-cp38-cp38}"
ARCH="${2:-x86_64}"

PYTHON="/opt/python/${TAG}/bin/python"
DOCKER_IMAGE="quay.io/pypa/manylinux1_${ARCH}"


PRE_CMD=
if [[ "${ARCH}" == "i686" ]]; then
    PRE_CMD=linux32
fi

SCRIPT=$(cat <<-END
${PRE_CMD}
set -ex
atexit() {
    chown -R ${uid}:${gid} /pwd/*
}
trap atexit EXIT
${PYTHON} -m pip install -U pip
${PYTHON} -m pip install cffi --prefer-binary
${PYTHON} -m pip install numpy --prefer-binary
${PYTHON} -m pip install pcpp
cd /pwd
make distclean
make wheel PYTHON=${PYTHON}
auditwheel repair --plat=manylinux1_${ARCH} -w dist dist/mulder*.whl
echo "cleaning"
make clean
chmod a+rw dist/mulder*.whl
END
)

docker run --mount type=bind,source=$(pwd),target=/pwd \
           ${DOCKER_IMAGE} /bin/bash -c "${SCRIPT}"
