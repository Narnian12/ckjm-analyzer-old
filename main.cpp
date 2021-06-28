#include <iostream>
#include <vector>

class Metric {
    int numClasses;
    int sumMetric;
public:
    Metric() {
        numClasses = 0;
        sumMetric = 0;
    }

    void incrementClass() {
        numClasses += 1;
    }

    void addMetric(int metricValue) {
        sumMetric += metricValue;
    }

    int computeMean() {
        return sumMetric / numClasses;
    }
};

int main() {
    std::cout << "WMC,DIT,NOC,CBO,RFC,LCOM,Ce,NPM\n";
    std::string className;

    std::vector<Metric> metricVec(8, Metric());

    int metric;

    while (std::cin >> className) {
        for (int i = 0; i < 8; ++i) {
            std::cin >> metric;
            metricVec[i].addMetric(metric);
            metricVec[i].incrementClass();
        }
    }

    for (auto avgMetric : metricVec) {
        std::cout << avgMetric.computeMean() << ",";
    }

    return 0;
}