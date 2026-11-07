#include "buntrail.h"
#include <assert.h>
#include <string.h>
int main(void) {
    assert(bt_abi_version()==1);
    BtEngine *engine=bt_engine_new();assert(engine);
    char *result=bt_call(engine,"{\"command\":\"document.open\",\"text\":\"rabbit 🐇\"}");
    assert(result && strstr(result,"\"ok\":true") && strstr(result,"rabbit"));
    bt_string_free(result);
    result=bt_call(engine,"{\"command\":\"document.edit\",\"id\":1,\"revision\":0,\"start\":0,\"end\":6,\"text\":\"Buntrail\"}");
    assert(result && strstr(result,"\"revision\":1"));bt_string_free(result);
    result=bt_call(engine,"{\"command\":\"document.close\",\"id\":1}");
    assert(result && strstr(result,"dirty_document"));bt_string_free(result);
    result=bt_call(engine,"{\"command\":\"service.start\",\"request\":{\"command\":\"fs.list\",\"path\":\".\"}}");
    assert(result && strstr(result,"\"ok\":true") && strstr(result,"\"id\":1")); bt_string_free(result);
    result=bt_call(engine,"{\"command\":\"service.cancel\",\"id\":1}");
    assert(result && strstr(result,"\"ok\":true")); bt_string_free(result);
    result=bt_call(engine,"{\"command\":\"service.poll\",\"id\":1}");
    assert(result && strstr(result,"\"done\":true")); bt_string_free(result);
    result=bt_call(engine,"{\"command\":\"service.release\",\"id\":1}");
    assert(result && strstr(result,"\"ok\":true")); bt_string_free(result);
    result=bt_call(engine,"{\"command\":\"service.poll\",\"id\":1}");
    assert(result && strstr(result,"not_found")); bt_string_free(result);
    bt_engine_free(engine);
}
