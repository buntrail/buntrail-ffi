#include "buntrail.h"
#include <cassert>
#include <cstring>
#include <memory>
#include <thread>
#include <vector>
int main() {
    std::unique_ptr<BtEngine, decltype(&bt_engine_free)> engine(bt_engine_new(),bt_engine_free);
    assert(engine);
    std::vector<std::thread> threads;
    for (int i=0;i<8;++i) threads.emplace_back([&]{
        for(int j=0;j<25;++j){
            char *result=bt_call(engine.get(),"{\"command\":\"document.open\",\"text\":\"parallel document\"}");
            assert(result && std::strstr(result,"\"ok\":true"));
            bt_string_free(result);
        }
    });
    for(auto &thread:threads) thread.join();
}
