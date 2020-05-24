import re
from datetime import datetime
from os import mkdir

print("Starting log splitter")
log_files = {}
file = open("../Logs/master.log", "r")
now = datetime.now().strftime("%Y-%m-%d %H:%M:%S")
path = "../Logs/" + now
mkdir(path)
misc_file = open(path+"/misc.log", "w")
temp = file.readline()
for line in file:
    regex = re.search("\([0-9]+\)", line)
    if regex is None:
        print("Regex failed?: ", line)
        misc_file.write(line)
    else:
        thread_num = regex.group().replace("(", "").replace(")", "")
        if thread_num not in log_files:
            log_files[thread_num] = open(path+"/thread-" + thread_num + ".log", "w")
            print(thread_num)
        log_files[thread_num].write(line)

file.close()
misc_file.close()
for thread_file in log_files.values():
    thread_file.close()
