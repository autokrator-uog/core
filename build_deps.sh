
unameOut="$(uname -s)"
case "${unameOut}" in
    Linux*)     machine=Linux;;
    Darwin*)    machine=Mac;;
    CYGWIN*)    machine=Cygwin;;
    MINGW*)     machine=MinGw;;
    *)          machine="UNKNOWN:${unameOut}"
esac

case "${machine}" in
  Mac)
    brew install libcouchbase;;
  
  Linux)
    # Ubuntu/Debian
    if [ command -v apt-get > /dev/null 2>&1 ]; then 
        wget http://packages.couchbase.com/releases/couchbase-release/couchbase-release-1.0-4-amd64.deb
        dpkg -i couchbase-release-1.0-4-amd64.deb
        apt-get update
        apt-get install libcouchbase-dev libcouchbase2-bin build-essential
    fi
    
    # any others can be added here...
    ;;
  *)
    echo "OS Unsupported - please install libcouchbase and libevent on your own."
esac
