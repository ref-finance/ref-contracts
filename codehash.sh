#/bin/bash
pushd $(dirname $0) > /dev/null

ORIG=releases/ref_exchange_release.wasm
DEST=res/ref_exchange_release.wasm

echo "aaa" | openssl dgst -sha256 -binary > /dev/null 2>&1
C1=$?
echo "aaa" | base58 > /dev/null 2>&1
C2=$?
if [ ${C1} -eq 0 ] && [ ${C2} -eq 0 ]; then
    a=`cat ${ORIG} | openssl dgst -sha256 -binary | base58`
    if [ ! -f "${DEST}" ]; then
        echo "Compute hashcode for ${ORIG} ..."
        echo "${a}"
        popd > /dev/null
        exit 0
    fi

    echo "Comparing ${ORIG} with ${DEST} ..."
    b=`cat ${DEST} | openssl dgst -sha256 -binary | base58`
    if [ "${a}" = "${b}" ]; then
        echo "In releases: ${a}"
        echo "In res:      ${b}"
        echo 'codehash is identical.'
    else
        echo "In releases: ${a}"
        echo "In res:      ${b}"
        echo 'codehash is different.'
    fi

    popd > /dev/null
    exit 0
fi

# docker mode
PYTHON_IMAGE=python
CONTAINER=python_base

echo 'Using docker to comparing boost_farming_release.wasm in releases folder with the one (if exist) in res folder ...'

docker ps -a | grep ${CONTAINER} > /dev/null || docker create \
    --cap-add=SYS_PTRACE --security-opt seccomp=unconfined \
    --name=${CONTAINER} \
    -w /host \
    -it \
    ${PYTHON_IMAGE} \
    /bin/bash

docker ps | grep ${CONTAINER} > /dev/null || docker start ${CONTAINER}

docker exec ${CONTAINER} ls -l | grep codehash.py && docker exec ${CONTAINER} rm codehash.py
docker exec ${CONTAINER} ls -l | grep release.wasm && docker exec ${CONTAINER} rm release.wasm
docker exec ${CONTAINER} ls -l | grep build.wasm && docker exec ${CONTAINER} rm build.wasm

docker cp scripts/codehash.py ${CONTAINER}:/host/codehash.py
docker cp releases/boost_farming_release.wasm ${CONTAINER}:/host/release.wasm
ls -l res | grep boost_farming_release.wasm > /dev/null && docker cp res/boost_farming_release.wasm ${CONTAINER}:/host/build.wasm

docker exec ${CONTAINER} pip3 install base58 > /dev/null 2>&1
docker exec ${CONTAINER} python3 codehash.py release.wasm build.wasm

docker exec ${CONTAINER} ls -l | grep release.wasm > /dev/null && docker exec ${CONTAINER} rm release.wasm
docker exec ${CONTAINER} ls -l | grep build.wasm > /dev/null && docker exec ${CONTAINER} rm build.wasm
docker exec ${CONTAINER} ls -l | grep codehash.py > /dev/null && docker exec ${CONTAINER} rm codehash.py

popd > /dev/null