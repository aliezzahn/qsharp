namespace Kata.Verification {
    open Microsoft.Quantum.Math;

    @EntryPoint()
    operation CheckSolution() : Bool {
        CheckTwoComplexOpsAreSame(Kata.ComplexDiv, DividedByC)
    }
}
