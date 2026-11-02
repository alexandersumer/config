// macOS-only filesystem fault injector. The Git shim arms one replacement after
// discovery; lstat returns the old metadata while a freshly created directory
// takes its path. No timestamps are forged and only this fixture's target moves.
#include <fcntl.h>
#include <limits.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/stat.h>
#include <unistd.h>

static void copy_entry(const char *from, const char *to, const char *name) {
    char source[PATH_MAX], target[PATH_MAX], bytes[4096];
    if (snprintf(source, sizeof(source), "%s/%s", from, name) < 0 ||
        snprintf(target, sizeof(target), "%s/%s", to, name) < 0) abort();
    int input = open(source, O_RDONLY);
    int output = open(target, O_WRONLY | O_CREAT | O_EXCL, 0600);
    if (input == -1 || output == -1) abort();
    ssize_t count;
    while ((count = read(input, bytes, sizeof(bytes))) > 0)
        if (write(output, bytes, count) != count) abort();
    if (count < 0) abort();
    close(input);
    close(output);
}
static int selection_lstat(const char *path, struct stat *metadata) {
    int result = lstat(path, metadata);
    const char *target = getenv("WORKCTL_RACE_TARGET");
    const char *armed = getenv("WORKCTL_RACE_ARMED");
    const char *done = getenv("WORKCTL_RACE_DONE");
    const char *saved = getenv("WORKCTL_RACE_SAVED");
    if (result == 0 && target && armed && done && saved &&
        strncmp(getprogname(), "workctl", 7) == 0 && strcmp(path, target) == 0 &&
        access(armed, F_OK) == 0) {
        int once = open(done, O_WRONLY | O_CREAT | O_EXCL, 0600);
        if (once != -1) {
            close(once);
            if (rename(target, saved) != 0 || mkdir(target, 0700) != 0) abort();
            copy_entry(saved, target, ".git");
            copy_entry(saved, target, "file");
            char precious[PATH_MAX];
            snprintf(precious, sizeof(precious), "%s/precious", target);
            int file = open(precious, O_WRONLY | O_CREAT | O_EXCL, 0600);
            if (file == -1 || write(file, "young replacement\n", 18) != 18) abort();
            close(file);
        }
    }
    return result;
}
__attribute__((used)) static struct { const void *replacement; const void *original; }
selection_interpose __attribute__((section("__DATA,__interpose"))) = {
    (const void *)selection_lstat, (const void *)lstat
};
