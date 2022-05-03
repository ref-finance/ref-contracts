import hashlib
import base58

def CalcFileSha256(filname):
    ''' calculate file sha256 '''
    with open(filname, "rb") as f:
        sha256obj = hashlib.sha256()
        sha256obj.update(f.read())
        bvalue = sha256obj.digest()
        # hash_value = sha256obj.hexdigest()
        return bvalue

if __name__ == '__main__':
    import sys
    release_file = sys.argv[1]
    build_file = sys.argv[2]
    
    code_hash_release = bytes.decode(base58.b58encode(CalcFileSha256(release_file)))
    print("In release, code hash:", code_hash_release)
    import os
    if os.path.exists(build_file):
        code_hash_build = bytes.decode(base58.b58encode(CalcFileSha256(release_file)))
        print("In res,     code hash:", code_hash_release)
        if code_hash_release == code_hash_build:
            print("OK, Code hash are identical.")
        else:
            print("Err!!! Two code hash are diffrent.")