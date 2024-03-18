/// # Sample
/// Quantinuum
///
/// # Description
/// This is sample Q# program meant to demonstrate the capabilities of Quantinuum systems.
namespace Quantinuum {
    open Microsoft.Quantum.Random;
    open Microsoft.Quantum.Math;
    open Microsoft.Quantum.Convert;
    open Microsoft.Quantum.Measurement;

    @EntryPoint()
    operation Main() : Result[] {
        // Branching bases on a measurement result is supported by Quantinuum.
        use q = Qubit();
        if M(q) == One {
            X(q);
        }

        // Creating a dynamic integer.
        // The compiler doesn't complain because Quantinuum supports dynamic integers.
        use register = Qubit[8];
        let results = MeasureEachZ(register);
        let dynamicInteger = ResultArrayAsInt(results);
        //let dynamicInteger = DrawRandomInt(0, 10);

        // However, dynamic integers can't be used just anywhere.
        let staticArray = [0, size = 10];
        //let dynamicallySizedArray = [0, size = dynamicInteger];

        // Even though Quantinuum supports dynamic integers, it doesn't support dynamic doubles.
        //let dynamicDouble = IntAsDouble(dynamicInteger);
        //let dynamicDouble = IntAsDouble(dynamicInteger);

        // But the compiler still allows classically calculated doubles because it can compute them before code generation.
        let classicalAngle = ArcSin(0.5);
        Rx(classicalAngle, q);
        //Rx(dynamicDouble / 256.0, q);

        // Recursive functions are special.
        // The compiler won't say anything if they are used with a classical value because it can compute it before code gen.
        let sum = GaussSumRecursiveFn(10);

        // However, if you try to call a recursive function with a dynamic value, the compiler will throw an error.
        //let sum = GaussSumRecursiveFn(dynamicInteger);

        // Recursive operations are even more special.
        // The compiler will throw an error no matter how you call them because an operation can allocate and measure
        // qubits, so it can't assume it can compute it even if its arguments are classical.
        //let sum = GaussSumRecursiveOp(10);
        //let sum = GaussSumRecursiveOp(dynamicInteger);

        return [];
    }

    function GaussSumRecursiveFn(n : Int) : Int {
        if n == 0 {
            0
        } else {
            n + GaussSumRecursiveFn(n - 1)
        }
    }

    //operation GaussSumRecursiveOp(n : Int) : Int {
    //    if n == 0 {
    //        0
    //    } else {
    //        n + GaussSumRecursiveOp(n - 1)
    //    }
    //}
}