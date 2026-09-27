from pathlib import Path
import hashlib
import json
import tempfile
import unittest
import generate as g


class GeneratorTests(unittest.TestCase):
    def test_exhaustive_spin_to_binary_identity(self):
        for denominator in (2,16):
            model=g.make(8,denominator,62001)
            for mask in range(256):
                x=[(mask>>i)&1 for i in range(8)]
                binary=model['offset']+sum(h*v for h,v in zip(model['linear'],x))+sum(j*x[a]*x[b] for a,b,j in model['pairs'])
                spin=sum((j//4)*(2*x[a]-1)*(2*x[b]-1) for a,b,j in model['pairs'])
                self.assertEqual(binary,spin)
                y=[1-v for v in x]
                self.assertEqual(binary,model['offset']+sum(h*v for h,v in zip(model['linear'],y))+sum(j*y[a]*y[b] for a,b,j in model['pairs']))

    def test_reproducible_coefficients_and_distinct_seed(self):
        self.assertEqual(g.encode(g.make(16,2,62001)),g.encode(g.make(16,2,62001)))
        self.assertNotEqual(g.encode(g.make(16,2,62001)),g.encode(g.make(16,2,62002)))
        for i,j,w in g.make(16,2,62001)['pairs']:
            self.assertLess(i,j);self.assertIn(w//4,list(range(-8,0))+list(range(1,9)));self.assertEqual(w%4,0)

    def test_declared_grid_and_seed_blocks(self):
        c=list(g.configurations());self.assertEqual(len(c),24)
        self.assertEqual([x['generator_seed'] for x in c],list(range(62001,62025)))
        self.assertEqual(len({x['id'] for x in c}),24)
        self.assertTrue(set(x['generator_seed'] for x in c).isdisjoint(range(61101,61111)))

    def test_frozen_corpus_regenerates_and_manifest_matches(self):
        here = Path(__file__).resolve().parent
        index = json.loads((here / 'instances.json').read_text())
        manifest = json.loads((here / 'manifest.json').read_text())
        self.assertEqual(index, manifest['instances'])
        self.assertEqual(hashlib.sha256((here / 'instances.json').read_bytes()).hexdigest(), manifest['input_index_sha256'])
        self.assertEqual(hashlib.sha256((here / 'generate.py').read_bytes()).hexdigest(), manifest['generator_sha256'])
        with tempfile.TemporaryDirectory() as tmp:
            regenerated = g.write_corpus(Path(tmp) / 'instances')
            self.assertEqual(index, regenerated)
            for entry in index:
                self.assertEqual((here / 'instances' / entry['file']).read_bytes(),
                                 (Path(tmp) / 'instances' / entry['file']).read_bytes())

    def test_reject_bad_arguments(self):
        for args in [(0,2,1),(513,2,1),(8,3,1),(8,2,-1),(True,2,1),(8,2.0,1),(8,2,True)]:
            with self.assertRaises(ValueError): g.make(*args)

    def test_existing_output_never_overwritten(self):
        with tempfile.TemporaryDirectory() as tmp:
            p=Path(tmp)/'data';p.mkdir();f=p/'sentinel';f.write_text('keep')
            with self.assertRaises(FileExistsError):g.write_corpus(p)
            self.assertEqual(f.read_text(),'keep')


if __name__=='__main__':unittest.main()
