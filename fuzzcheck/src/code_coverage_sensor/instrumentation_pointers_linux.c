extern int __llvm_profile_runtime = 0;

extern unsigned long int __start___llvm_prf_cnts;
extern unsigned long int __stop___llvm_prf_cnts;

extern char __start___llvm_prf_data;
extern char __stop___llvm_prf_data;

char * get_start_prf_data() {
    return &__start___llvm_prf_data;
}
char *get_end_prf_data() {
    return &__stop___llvm_prf_data;
}

unsigned long int * get_start_instrumentation_counters() {
    return &__start___llvm_prf_cnts;
}
unsigned long int * get_end_instrumentation_counters() {
    return &__stop___llvm_prf_cnts;
}
