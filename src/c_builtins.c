#import <stdint.h>

void* return_address() {
	return __builtin_return_address(1);
}
